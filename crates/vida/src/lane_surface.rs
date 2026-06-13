use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use serde::Serialize;
use time::format_description::well_known::Rfc3339;

use crate::contract_profile_adapter::render_operator_contract_envelope;
use crate::taskflow_task_bridge::proxy_state_dir;
use crate::{state_store::StateStore, ProxyArgs};

const LANE_SURFACE_LOCK_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Serialize)]
struct LaneEnvelope {
    surface: &'static str,
    status: &'static str,
    trace_id: Option<String>,
    workflow_class: Option<String>,
    risk_tier: Option<String>,
    artifact_refs: serde_json::Value,
    next_action: Option<LaneNextAction>,
    recommended_command: Option<String>,
    recommended_surface: Option<String>,
    next_actions: Vec<String>,
    blocker_codes: Vec<String>,
    run_id: String,
    lane_id: Option<String>,
    runtime_role: Option<String>,
    lane_status: String,
    selected_backend: Option<String>,
    dispatch_status: String,
    operator_session_projection: serde_json::Value,
    supersedes_receipt_id: Option<String>,
    exception_path_receipt_id: Option<String>,
    exception_path_metadata_path: Option<String>,
    exception_path_metadata: Option<ExceptionTakeoverMetadata>,
    root_local_write_allowed: bool,
    owned_write_scope: Vec<String>,
    root_local_write_allowed_for_only_these_paths: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct LaneNextAction {
    command: String,
    surface: String,
    reason: String,
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

#[derive(Serialize)]
struct LaneTakeoverReadyEnvelope {
    surface: &'static str,
    status: &'static str,
    trace_id: Option<String>,
    workflow_class: Option<String>,
    risk_tier: Option<String>,
    artifact_refs: serde_json::Value,
    run_id: String,
    lane_status: String,
    dispatch_status: String,
    takeover_state: String,
    takeover_ready: bool,
    root_local_write_allowed: bool,
    owned_write_scope: Vec<String>,
    recommended_command: Option<String>,
    recommended_surface: Option<String>,
    next_action: Option<LaneNextAction>,
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
    TakeoverReady {
        run_id: &'a str,
        as_json: bool,
    },
    Complete {
        run_id: &'a str,
        receipt_id: &'a str,
        host_bridge_request: Option<&'a str>,
        host_agent_id: Option<&'a str>,
        host_bridge_summary: Option<&'a str>,
        state_dir: Option<&'a str>,
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
        activate: bool,
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
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    dispatch_target: Option<String>,
    #[serde(default)]
    dispatch_packet_path: Option<String>,
    #[serde(default)]
    source_exception_path_receipt_id: Option<String>,
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

impl ExceptionTakeoverMetadata {
    fn bind_to_receipt(&mut self, receipt: &crate::state_store::RunGraphDispatchReceipt) {
        self.run_id = Some(receipt.run_id.clone());
        self.dispatch_target = Some(receipt.dispatch_target.clone());
        self.dispatch_packet_path = receipt.dispatch_packet_path.clone();
        self.source_exception_path_receipt_id = receipt.exception_path_receipt_id.clone();
    }

    fn validate_for_receipt(
        &self,
        receipt: &crate::state_store::RunGraphDispatchReceipt,
    ) -> Result<(), String> {
        let run_id = self
            .run_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "exception takeover metadata is missing receipt-bound `run_id`; record a fresh exception takeover for the current lane before superseding".to_string()
            })?;
        if run_id != receipt.run_id {
            return Err(format!(
                "exception takeover metadata run_id `{run_id}` does not match current lane `{}`",
                receipt.run_id
            ));
        }

        let dispatch_target = self
            .dispatch_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "exception takeover metadata is missing receipt-bound `dispatch_target`; record a fresh exception takeover for the current lane before superseding".to_string()
            })?;
        if dispatch_target != receipt.dispatch_target {
            return Err(format!(
                "exception takeover metadata dispatch_target `{dispatch_target}` does not match current lane target `{}`",
                receipt.dispatch_target
            ));
        }

        let source_receipt_id = self
            .source_exception_path_receipt_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "exception takeover metadata is missing receipt-bound `source_exception_path_receipt_id`; record a fresh exception takeover for the current lane before superseding".to_string()
            })?;
        let current_exception_receipt = receipt
            .exception_path_receipt_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "current lane receipt is missing exception_path_receipt_id; record exception takeover before superseding".to_string()
            })?;
        if source_receipt_id != current_exception_receipt {
            return Err(format!(
                "exception takeover metadata source receipt `{source_receipt_id}` does not match current exception receipt `{current_exception_receipt}`"
            ));
        }

        Ok(())
    }

    fn matches_summary(
        &self,
        summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    ) -> bool {
        let run_id_matches = self
            .run_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| value == summary.run_id);
        let target_matches = self
            .dispatch_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| value == summary.dispatch_target);
        let source_receipt_matches = self
            .source_exception_path_receipt_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| {
                summary
                    .exception_path_receipt_id
                    .as_deref()
                    .is_some_and(|summary_value| value == summary_value)
            });
        run_id_matches && target_matches && source_receipt_matches
    }
}

fn lane_usage() -> &'static str {
    "Usage: vida lane show <run-id> [--json]\n       vida lane show --latest [--json]\n       vida lane takeover-ready <run-id> [--json]\n       vida lane complete <run-id> --receipt-id <id> [--host-bridge-request <path>] [--host-agent-id <id>] [--host-bridge-summary <text>] [--state-dir <path>] [--json]\n       vida lane retire <run-id> --receipt-id <id> --reason <text> [--json]\n       vida lane exception-takeover <run-id> --receipt-id <id> --reason-class <class> --active-bounded-unit <unit> --owned-write-scope <path> [--owned-write-scope <path> ...] --why-delegated-path-not-lawful <text> --why-local-write-safe <text> --return-to-normal-when <text> --verification-step <text> [--verification-step <text> ...] [--activate] [--json]\n       vida lane supersede <run-id> --receipt-id <id> [--json]\n       vida lane reclaim --completed --host-agents [--json]\n\nOptions:\n  --receipt-id <id>              Receipt id that proves the lane mutation source\n  --reason <text>                Human-readable retire reason\n  --host-bridge-request <path>   Host bridge request artifact to complete\n  --host-agent-id <id>           Parent host agent id that executed the bridge request\n  --host-bridge-summary <text>   Completion summary from the parent host adapter\n  --state-dir <path>             Override the TaskFlow state directory for this lane mutation\n  --reason-class <class>         Exception takeover reason class\n  --active-bounded-unit <unit>   Bounded unit authorized by the exception path\n  --owned-write-scope <path>     Receipt-bound write scope; may be repeated\n  --verification-step <text>     Verification step for exception takeover; may be repeated\n  --activate                     Activate the exception takeover immediately\n  --completed                    Reclaim completed lanes\n  --host-agents                  Include host-agent lane handles during reclaim\n  --json                         Emit machine-readable JSON output\n  -h, --help                     Print help"
}

fn lane_retire_help() -> &'static str {
    "Usage: vida lane retire <run-id> --receipt-id <id> --reason <text> [--json]\n\nPurpose:\n  Retire a stale or blocked lane when runtime recovery has identified the run as safe to remove from active continuation.\n\nOptions:\n  --receipt-id <id>   Receipt id that proves the lane mutation source\n  --reason <text>     Human-readable retire reason\n  --json              Emit machine-readable JSON output\n  -h, --help          Print help"
}

fn lane_help_text(args: &[String]) -> &'static str {
    if args
        .first()
        .is_some_and(|arg| matches!(arg.as_str(), "retire"))
    {
        lane_retire_help()
    } else {
        lane_usage()
    }
}

fn lane_help_requested(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
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
        [head, run_id, rest @ ..] if head == "takeover-ready" => {
            let mut as_json = false;
            for arg in rest {
                match arg.as_str() {
                    "--json" => as_json = true,
                    _ => return Err(lane_usage().to_string()),
                }
            }
            Ok(LaneCommand::TakeoverReady { run_id, as_json })
        }
        [head, run_id, rest @ ..] if head == "complete" => {
            let mut as_json = false;
            let mut receipt_id = None;
            let mut host_bridge_request = None;
            let mut host_agent_id = None;
            let mut host_bridge_summary = None;
            let mut state_dir = None;
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
                    "--host-bridge-request" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        host_bridge_request = Some(value.as_str());
                        index += 2;
                    }
                    "--host-agent-id" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        host_agent_id = Some(value.as_str());
                        index += 2;
                    }
                    "--host-bridge-summary" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        host_bridge_summary = Some(value.as_str());
                        index += 2;
                    }
                    "--state-dir" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        state_dir = Some(value.as_str());
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
                host_bridge_request,
                host_agent_id,
                host_bridge_summary,
                state_dir,
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
            let mut activate = false;
            let mut index = 0;
            while index < rest.len() {
                match rest[index].as_str() {
                    "--json" => {
                        as_json = true;
                        index += 1;
                    }
                    "--activate" => {
                        activate = true;
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
                run_id: None,
                dispatch_target: None,
                dispatch_packet_path: None,
                source_exception_path_receipt_id: None,
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
                activate,
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
    operator_session_projection: serde_json::Value,
    blocked: bool,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
) -> LaneEnvelope {
    build_lane_envelope_with_owned_scope(
        summary,
        status,
        exception_path_metadata_path,
        exception_path_metadata,
        operator_session_projection,
        blocked,
        blocker_codes,
        next_actions,
        &[],
    )
}

fn build_lane_envelope_with_owned_scope(
    summary: crate::state_store::RunGraphDispatchReceiptSummary,
    status: Option<crate::state_store::RunGraphStatus>,
    exception_path_metadata_path: Option<String>,
    exception_path_metadata: Option<ExceptionTakeoverMetadata>,
    operator_session_projection: serde_json::Value,
    blocked: bool,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    owned_write_scope_hint: &[String],
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
    let root_local_write_allowed = !root_local_write_allowed_for_only_these_paths.is_empty();
    let owned_write_scope = root_local_write_allowed_for_only_these_paths.clone();
    let next_action = lane_ready_downstream_next_action(&summary, blocked).or_else(|| {
        lane_blocked_next_action(
            &summary,
            status.as_ref(),
            blocked,
            &next_actions,
            owned_write_scope_hint,
        )
    });
    let recommended_command = next_action.as_ref().map(|action| action.command.clone());
    let recommended_surface = next_action.as_ref().map(|action| action.surface.clone());
    let selected_backend = status
        .as_ref()
        .map(|status| status.selected_backend.clone())
        .or(summary.selected_backend.clone());
    let artifact_refs = serde_json::json!({
        "latest_run_graph_dispatch_receipt_id": run_id.clone(),
        "exception_path_receipt_id": exception_path_receipt_id.clone(),
        "exception_path_metadata_path": exception_path_metadata_path.clone(),
        "root_local_write_allowed": root_local_write_allowed,
        "owned_write_scope": owned_write_scope.clone(),
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
        next_action,
        recommended_command,
        recommended_surface,
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
        operator_session_projection,
        supersedes_receipt_id,
        exception_path_receipt_id,
        exception_path_metadata_path,
        root_local_write_allowed,
        owned_write_scope,
        root_local_write_allowed_for_only_these_paths,
        exception_path_metadata,
    }
}

fn lane_takeover_state_label(envelope: &LaneEnvelope) -> &'static str {
    if envelope.root_local_write_allowed {
        return "active";
    }
    match envelope.recommended_surface.as_deref() {
        Some("vida lane exception-takeover") => "ready_to_record",
        Some("vida task show") => "missing_owned_scope",
        Some("vida lane supersede") => "supersession_required",
        _ => "not_ready",
    }
}

fn build_lane_takeover_ready_envelope(envelope: LaneEnvelope) -> LaneTakeoverReadyEnvelope {
    let takeover_state = lane_takeover_state_label(&envelope).to_string();
    let takeover_ready = matches!(takeover_state.as_str(), "active" | "ready_to_record");
    let reason = match takeover_state.as_str() {
        "active" => "exception takeover is active; local writes are lawful only inside owned_write_scope",
        "ready_to_record" => {
            "delegated lane is blocked and lane surface has enough evidence to record exception takeover"
        }
        "missing_owned_scope" => {
            "delegated lane is blocked but owned write scope must be inspected before takeover"
        }
        "supersession_required" => {
            "exception receipt is recorded but supersession is required before local writes become lawful"
        }
        _ => "lane is not currently ready for exception takeover",
    }
    .to_string();
    let artifact_refs = serde_json::json!({
        "latest_run_graph_dispatch_receipt_id": envelope.run_id.clone(),
        "root_local_write_allowed": envelope.root_local_write_allowed,
        "owned_write_scope": envelope.owned_write_scope.clone(),
        "recommended_command": envelope.recommended_command.clone(),
        "recommended_surface": envelope.recommended_surface.clone(),
    });
    let operator_contracts = render_operator_contract_envelope(
        if takeover_ready { "pass" } else { "blocked" },
        if takeover_ready {
            Vec::new()
        } else {
            envelope.blocker_codes.clone()
        },
        envelope.next_actions.clone(),
        artifact_refs,
    );
    let status = if operator_contracts["status"].as_str() == Some("blocked") {
        "blocked"
    } else {
        "pass"
    };
    LaneTakeoverReadyEnvelope {
        surface: "vida lane takeover-ready",
        status,
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
        run_id: envelope.run_id,
        lane_status: envelope.lane_status,
        dispatch_status: envelope.dispatch_status,
        takeover_state,
        takeover_ready,
        root_local_write_allowed: envelope.root_local_write_allowed,
        owned_write_scope: envelope.owned_write_scope,
        recommended_command: envelope.recommended_command,
        recommended_surface: envelope.recommended_surface,
        next_action: envelope.next_action,
        next_actions: envelope.next_actions,
        blocker_codes: envelope.blocker_codes,
        reason,
    }
}

fn active_exception_write_scope(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    exception_path_metadata: Option<&ExceptionTakeoverMetadata>,
) -> Vec<String> {
    if lane_summary_is_terminal_completed(summary) {
        return Vec::new();
    }
    if !crate::release1_contracts::exception_takeover_state(
        summary.exception_path_receipt_id.as_deref(),
        summary.supersedes_receipt_id.as_deref(),
        None,
    )
    .is_active()
    {
        return Vec::new();
    }
    exception_path_metadata
        .filter(|metadata| metadata.matches_summary(summary))
        .map(|metadata| metadata.owned_write_scope.clone())
        .unwrap_or_default()
}

fn lane_recommended_surface_for_command(command: &str) -> String {
    if command.starts_with("vida agent-init") {
        return "vida agent-init".to_string();
    }
    command
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
}

fn lane_ready_downstream_next_action(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    blocked: bool,
) -> Option<LaneNextAction> {
    if blocked || !lane_summary_has_ready_downstream_handoff(summary) {
        return None;
    }
    let command =
        crate::continuation_binding_summary::downstream_dispatch_command_for_summary(summary)?;
    Some(LaneNextAction {
        command: command.clone(),
        surface: lane_recommended_surface_for_command(&command),
        reason: format!(
            "continue the ready downstream `{}` handoff with the persisted dispatch packet",
            summary
                .downstream_dispatch_target
                .as_deref()
                .unwrap_or("next")
        ),
    })
}

fn lane_blocked_next_action(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    status: Option<&crate::state_store::RunGraphStatus>,
    blocked: bool,
    next_actions: &[String],
    owned_write_scope_hint: &[String],
) -> Option<LaneNextAction> {
    if !blocked {
        return None;
    }
    if let Some(action) = pending_host_bridge_next_action(summary, status) {
        return Some(action);
    }
    if let Some(receipt_id) = summary
        .exception_path_receipt_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|_| summary.supersedes_receipt_id.is_none())
    {
        let command = format!(
            "vida lane supersede {} --receipt-id {}",
            crate::shell_quote(summary.run_id.trim()),
            crate::shell_quote(receipt_id)
        );
        return Some(LaneNextAction {
            surface: lane_recommended_surface_for_command(&command),
            command,
            reason: "activate the recorded exception-path receipt before treating local recovery as lawful".to_string(),
        });
    }
    let open_cycle_recovery = next_actions.iter().any(|action| {
        action.contains("record structured exception takeover")
            || action.contains("delegated cycle is still open")
            || action.contains("root-local write remains blocked")
    });
    if !open_cycle_recovery {
        return None;
    }
    let receipt_id = format!("{}-exception-takeover", summary.run_id.trim());
    let reason_class = summary
        .blocker_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            summary
                .downstream_dispatch_blockers
                .iter()
                .map(String::as_str)
                .map(str::trim)
                .find(|value| !value.is_empty())
        })
        .unwrap_or("blocked_open_delegated_cycle");
    let active_node = status
        .map(|status| status.active_node.as_str())
        .unwrap_or_else(|| summary.dispatch_target.as_str())
        .trim();
    let active_node = if active_node.is_empty() {
        "delegated-lane"
    } else {
        active_node
    };
    let task_id = status
        .map(|status| status.task_id.as_str())
        .unwrap_or_else(|| summary.run_id.as_str())
        .trim();
    let task_id = if task_id.is_empty() {
        summary.run_id.trim()
    } else {
        task_id
    };
    let active_bounded_unit = format!("{task_id}:{active_node}:exception-takeover");
    let owned_write_scope_args = owned_write_scope_hint
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("--owned-write-scope {}", crate::shell_quote(value)))
        .collect::<Vec<_>>();
    if owned_write_scope_args.is_empty() {
        let command = format!("vida task show {}", crate::shell_quote(task_id));
        return Some(LaneNextAction {
            surface: "vida task show".to_string(),
            command,
            reason: "inspect the active task owned paths before recording exception takeover; lane show cannot emit a takeover command without a concrete owned write scope".to_string(),
        });
    }
    let why_delegated_not_lawful = format!(
        "delegated lane is blocked for run {} by {}",
        summary.run_id.trim(),
        reason_class
    );
    let why_local_safe = format!(
        "bounded exception recovery is limited to the active {} unit and declared owned write scope",
        active_node
    );
    let return_to_normal =
        "after focused proof, release install, task closure, and lane completion";
    let verification_step = format!(
        "run focused proof for {} before local write closure",
        active_bounded_unit
    );
    let command = format!(
        "vida lane exception-takeover {} --receipt-id {} --reason-class {} --active-bounded-unit {} {} --why-delegated-path-not-lawful {} --why-local-write-safe {} --return-to-normal-when {} --verification-step {}",
        crate::shell_quote(summary.run_id.trim()),
        crate::shell_quote(&receipt_id),
        crate::shell_quote(reason_class),
        crate::shell_quote(&active_bounded_unit),
        owned_write_scope_args.join(" "),
        crate::shell_quote(&why_delegated_not_lawful),
        crate::shell_quote(&why_local_safe),
        crate::shell_quote(return_to_normal),
        crate::shell_quote(&verification_step),
    );
    Some(LaneNextAction {
        surface: lane_recommended_surface_for_command(&command),
        command,
        reason: "record bounded exception-path evidence for the dispatch blocker before local recovery work".to_string(),
    })
}

fn pending_host_bridge_next_action(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    status: Option<&crate::state_store::RunGraphStatus>,
) -> Option<LaneNextAction> {
    let retryable_blocked_host_bridge = summary.dispatch_status == "blocked"
        && (summary
            .blocker_code
            .as_deref()
            .is_some_and(host_bridge_completion_retryable_blocker)
            || summary
                .downstream_dispatch_blockers
                .iter()
                .any(|blocker| host_bridge_completion_retryable_blocker(blocker)));
    if summary.dispatch_status != "bridge_request_pending" && !retryable_blocked_host_bridge {
        return None;
    }
    if let Some(request_path) = status
        .filter(|status| status.active_node.trim() != summary.dispatch_target.trim())
        .and_then(|status| {
            host_bridge_request_path_for_run_target(summary, status.active_node.trim())
        })
    {
        return Some(host_bridge_next_action_for_request_path(request_path));
    }
    if let Some(request_path) = blocked_source_target_from_summary_packet(summary)
        .and_then(|target| host_bridge_request_path_for_run_target(summary, &target))
    {
        return Some(host_bridge_next_action_for_request_path(request_path));
    }
    if retryable_blocked_host_bridge {
        if let Some(request_path) =
            host_bridge_request_path_for_run_target(summary, summary.dispatch_target.trim())
        {
            return Some(host_bridge_next_action_for_request_path(request_path));
        }
    }
    let state_root = host_bridge_state_root_from_receipt_summary(summary)?;
    let dispatch_result_path = summary.dispatch_result_path.as_deref()?.trim();
    if dispatch_result_path.is_empty() {
        return None;
    }
    let result_path =
        crate::runtime_dispatch_state::normalize_persisted_runtime_path(dispatch_result_path);
    let result_path =
        canonicalize_existing_regular_state_path(&state_root, &result_path, "dispatch result")
            .ok()?;
    let result = read_host_bridge_request_at_path(&result_path).ok()?;
    let request = host_bridge_request_object(&result)?;
    let request_path = host_bridge_path_string(request, "request_path")
        .ok()?
        .to_string();
    Some(host_bridge_next_action_for_request_path(request_path))
}

fn host_bridge_next_action_for_request_path(request_path: String) -> LaneNextAction {
    let command = format!(
        "vida agent host-bridge --request {}",
        crate::shell_quote(&request_path)
    );
    LaneNextAction {
        surface: "vida agent host-bridge".to_string(),
        command,
        reason: "complete or retry the parent-host bridge before considering exception recovery"
            .to_string(),
    }
}

fn host_bridge_request_path_for_run_target(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    dispatch_target: &str,
) -> Option<String> {
    let state_root = host_bridge_state_root_from_receipt_summary(summary)?;
    let mut scan_dirs = vec![
        state_root.join("host-tool-bridge").join("requests"),
        state_root
            .join("runtime-consumption")
            .join("host-tool-bridge"),
    ];
    collect_host_bridge_scan_dirs(&state_root, 0, &mut scan_dirs);
    scan_dirs.sort();
    scan_dirs.dedup();
    let mut candidates = scan_dirs
        .into_iter()
        .filter_map(|request_dir| std::fs::read_dir(request_dir).ok())
        .flat_map(|entries| entries.flatten())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                return None;
            }
            let canonical_path =
                canonicalize_existing_regular_state_path(&state_root, &path, "request").ok()?;
            let request = read_host_bridge_request_at_path(&canonical_path).ok()?;
            let run_matches = request
                .get("run_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|run_id| run_id.trim() == summary.run_id.trim());
            let target_matches = request
                .get("dispatch_target")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .is_some_and(|target| target == dispatch_target);
            let pending = request
                .get("status")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|status| status.trim() == "pending");
            let request_path = host_bridge_path_string(&request, "request_path")
                .map(str::to_string)
                .unwrap_or_else(|_| path.display().to_string());
            let retryable_blocked = request
                .get("status")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .is_some_and(|status| matches!(status, "blocked" | "retryable_blocked"))
                && host_bridge_request_has_retryable_completion_evidence(
                    &state_root,
                    &request_path,
                );
            if run_matches && (pending || retryable_blocked) && target_matches {
                let score = 2 + i32::from(retryable_blocked);
                Some((score, request_path))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.pop().map(|(_, path)| path)
}

fn collect_host_bridge_scan_dirs(
    directory: &std::path::Path,
    depth: usize,
    scan_dirs: &mut Vec<std::path::PathBuf>,
) {
    if depth >= 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path
            .components()
            .any(|component| component.as_os_str() == ".git")
        {
            continue;
        }
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.contains("host-tool-bridge") || name == "requests")
        {
            scan_dirs.push(path.clone());
        }
        collect_host_bridge_scan_dirs(&path, depth + 1, scan_dirs);
    }
}

fn host_bridge_state_root_from_receipt_summary(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> Option<std::path::PathBuf> {
    let path = summary
        .dispatch_result_path
        .as_deref()
        .or(summary.dispatch_packet_path.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(path);
    for ancestor in path.ancestors() {
        if ancestor.file_name().and_then(|value| value.to_str()) == Some("runtime-consumption") {
            return ancestor.parent().map(std::path::Path::to_path_buf);
        }
        if ancestor.file_name().and_then(|value| value.to_str()) == Some("state") {
            let data_dir = ancestor.parent()?;
            let vida_dir = data_dir.parent()?;
            if data_dir.file_name().and_then(|value| value.to_str()) == Some("data")
                && vida_dir.file_name().and_then(|value| value.to_str()) == Some(".vida")
            {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

fn blocked_source_target_from_summary_packet(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> Option<String> {
    let packet_path = summary.dispatch_packet_path.as_deref()?.trim();
    if packet_path.is_empty() {
        return None;
    }
    let project_root = std::env::current_dir().ok()?;
    let packet =
        crate::status_surface::dispatch_packet_json_from_project_path(&project_root, packet_path)?;
    let source_dispatch_target = packet
        .get("source_dispatch_target")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if source_dispatch_target == summary.dispatch_target.trim() {
        return None;
    }
    let source_dispatch_status = packet
        .get("source_dispatch_status")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let source_blocked = packet
        .get("source_blocker_code")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.trim().is_empty());
    let downstream_ready = packet
        .get("downstream_dispatch_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let downstream_blocked = packet
        .get("downstream_dispatch_blockers")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|blockers| !blockers.is_empty());
    if source_dispatch_status == "executed"
        && !source_blocked
        && downstream_ready
        && !downstream_blocked
    {
        return None;
    }
    Some(source_dispatch_target.to_string())
}

async fn task_owned_write_scope_for_status(
    store: &StateStore,
    status: Option<&crate::state_store::RunGraphStatus>,
) -> Vec<String> {
    let Some(task_id) = status
        .map(|status| status.task_id.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Vec::new();
    };
    match store.show_task(task_id).await {
        Ok(task) => task
            .planner_metadata
            .owned_paths
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

async fn retired_closed_task_status_for_show(
    store: &StateStore,
    status: Option<&crate::state_store::RunGraphStatus>,
) -> Option<crate::state_store::RunGraphStatus> {
    let status = status?;
    match crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(store, status)
        .await
    {
        Ok(verdict) if verdict.task_closed_stale_run() => {
            Some(retired_closed_task_run_graph_status(status.clone()))
        }
        _ => None,
    }
}

fn retired_closed_task_summary_for_show(
    mut summary: crate::state_store::RunGraphDispatchReceiptSummary,
) -> crate::state_store::RunGraphDispatchReceiptSummary {
    summary.dispatch_status = "executed".to_string();
    summary.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
    summary.blocker_code = None;
    summary.exception_path_receipt_id = None;
    summary.supersedes_receipt_id = None;
    summary.downstream_dispatch_target = None;
    summary.downstream_dispatch_command = None;
    summary.downstream_dispatch_packet_path = None;
    summary.downstream_dispatch_ready = false;
    summary.downstream_dispatch_blockers.clear();
    summary.downstream_dispatch_status = Some("retired_closed_task_run".to_string());
    summary.downstream_dispatch_active_target = Some("closure".to_string());
    summary.downstream_dispatch_last_target = Some("closure".to_string());
    summary
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
    let has_blocker_code = summary
        .blocker_code
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    matches!(dispatch_status.as_str(), "blocked" | "failed")
        || matches!(lane_status.as_str(), "lane_blocked" | "lane_failed")
        || has_blocker_code
        || has_downstream_blockers
}

fn lane_summary_has_ready_downstream_handoff(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> bool {
    summary.dispatch_status == "executed"
        && summary.blocker_code.is_none()
        && summary.downstream_dispatch_ready
        && summary.downstream_dispatch_blockers.is_empty()
        && summary
            .downstream_dispatch_status
            .as_deref()
            .is_some_and(|status| status.eq_ignore_ascii_case("packet_ready"))
}

fn recovery_delegated_cycle_open(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> bool {
    recovery.is_some_and(|recovery| {
        recovery.delegation_gate.local_exception_takeover_gate == "blocked_open_delegated_cycle"
            || recovery.delegation_gate.delegated_cycle_open
    })
}

fn lane_summary_is_terminal_completed(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> bool {
    summary.lane_status == crate::LaneStatus::LaneCompleted.as_str()
        && summary.dispatch_status == "executed"
        && summary.blocker_code.is_none()
        && summary
            .downstream_dispatch_blockers
            .iter()
            .all(|value| value.trim().is_empty())
}

fn lane_summary_raw_blocker_codes(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    include_downstream: bool,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    if crate::runtime_dispatch_receipt_helpers::dispatch_summary_is_materialization_only_blocked_task_ensure(summary)
    {
        blocker_codes.push("internal_activation_view_only".to_string());
    }
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
    for blocker_code in blocker_codes
        .iter()
        .map(|value| value.trim())
        .filter(|value| lane_show_preserves_raw_blocker_code(value))
    {
        if !canonical_codes.iter().any(|code| code == blocker_code) {
            canonical_codes.push(blocker_code.to_string());
        }
    }
    let has_uncanonical_dispatch_blocker = blocker_codes.iter().any(|value| {
        !value.trim().is_empty()
            && !lane_show_preserves_raw_blocker_code(value.trim())
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

fn lane_show_preserves_raw_blocker_code(blocker_code: &str) -> bool {
    matches!(
        blocker_code,
        "host_tool_bridge_adapter_required"
            | "internal_activation_view_only"
            | "implementation_artifact_changed_files_missing"
            | "implementation_artifact_contract_invalid"
            | "implementation_artifacts_missing"
            | "implementation_attempt_scope_guard_violation"
            | "lane_completion_blocked_by_summary"
    )
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
        "Inspect {lane} for run `{}` with `vida taskflow recovery status {}` and keep the blocked dispatch result from `vida lane show {}` as evidence.",
        summary.run_id, run_id, run_id
    );
    if recovery_delegated_cycle_open(recovery) {
        action.push_str(&format!(
            " If no receipt-backed delegated completion exists, record structured exception takeover for run `{}` with a concrete receipt id, active bounded unit, and owned write scope, then supersede the lane with the same receipt id before local recovery work.",
            summary.run_id
        ));
    } else {
        action.push_str(&format!(
            " If the dispatch blocker has already been resolved, rerun `vida taskflow consume continue --run-id {}` to refresh continuation evidence.",
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

    if takeover_state.is_active() {
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

    let completed_resolves_recovery_open_cycle = summary.lane_status
        == crate::LaneStatus::LaneCompleted.as_str()
        && summary.dispatch_status == "executed"
        && summary.blocker_code.is_none();
    let recovery_open_blocks_completed =
        recovery_delegated_cycle_open(recovery) && !completed_resolves_recovery_open_cycle;
    let completed_has_blocked_downstream = summary.lane_status
        == crate::LaneStatus::LaneCompleted.as_str()
        && (lane_summary_dispatch_is_blocked(summary)
            || summary
                .downstream_dispatch_blockers
                .iter()
                .any(|value| !value.trim().is_empty())
            || recovery_open_blocks_completed);
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
    let recovery_open_blocks_lane =
        recovery_open && !lane_summary_has_ready_downstream_handoff(summary);
    let mut blocked = lane_summary_dispatch_is_blocked(summary) || recovery_open_blocks_lane;
    let mut blocker_codes = lane_summary_raw_blocker_codes(summary, blocked);
    let mut next_actions = Vec::new();
    if recovery_open_blocks_lane {
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
                    "Exception-path receipt recorded for lane `{}` but no concrete receipt id is available; inspect `vida lane show {}` and recover the recorded receipt before supersession.",
                    summary.run_id, run_id
                )
            } else {
                let receipt_id = crate::shell_quote(receipt_id.trim());
                format!(
                    "Exception-path receipt recorded; record explicit supersession with `vida lane supersede {} --receipt-id {}` before local write becomes active.",
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

fn derive_lane_show_truth_with_exception_metadata(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    exception_path_metadata: Option<&ExceptionTakeoverMetadata>,
) -> LaneShowTruth {
    if lane_summary_is_terminal_completed(summary) {
        return LaneShowTruth {
            blocked: false,
            blocker_codes: Vec::new(),
            next_actions: Vec::new(),
        };
    }

    if crate::release1_contracts::exception_takeover_state(
        summary.exception_path_receipt_id.as_deref(),
        summary.supersedes_receipt_id.as_deref(),
        recovery_takeover_gate(recovery),
    )
    .is_active()
        && !active_exception_write_scope(summary, exception_path_metadata).is_empty()
    {
        return LaneShowTruth {
            blocked: false,
            blocker_codes: Vec::new(),
            next_actions: Vec::new(),
        };
    }
    derive_lane_show_truth(summary, recovery)
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
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "operator_session_projection",
        &crate::operator_session_projection::projection_plain_summary(
            &envelope.operator_session_projection,
        ),
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
    if let Some(command) = envelope.recommended_command.as_deref() {
        crate::print_surface_line(crate::RenderMode::Plain, "recommended_command", command);
    }
    if let Some(surface) = envelope.recommended_surface.as_deref() {
        crate::print_surface_line(crate::RenderMode::Plain, "recommended_surface", surface);
    }
    if let Some(next_action) = envelope.next_action.as_ref() {
        crate::print_surface_line(crate::RenderMode::Plain, "next_action", &next_action.reason);
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
        "root_local_write_allowed",
        &envelope.root_local_write_allowed.to_string(),
    );
    if !envelope.owned_write_scope.is_empty() {
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "owned_write_scope",
            &envelope.owned_write_scope.join(", "),
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

fn emit_lane_takeover_ready_envelope(
    envelope: &LaneTakeoverReadyEnvelope,
    as_json: bool,
) -> ExitCode {
    if crate::surface_render::print_surface_json(
        envelope,
        as_json,
        "lane takeover-ready surface should serialize",
    ) {
        return if envelope.status == "pass" {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(2)
        };
    }

    crate::print_surface_header(crate::RenderMode::Plain, envelope.surface);
    crate::print_surface_line(crate::RenderMode::Plain, "status", envelope.status);
    crate::print_surface_line(crate::RenderMode::Plain, "run_id", &envelope.run_id);
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "takeover_state",
        &envelope.takeover_state,
    );
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "takeover_ready",
        &envelope.takeover_ready.to_string(),
    );
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "root_local_write_allowed",
        &envelope.root_local_write_allowed.to_string(),
    );
    if !envelope.owned_write_scope.is_empty() {
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "owned_write_scope",
            &envelope.owned_write_scope.join(", "),
        );
    }
    if let Some(command) = envelope.recommended_command.as_deref() {
        crate::print_surface_line(crate::RenderMode::Plain, "recommended_command", command);
    }
    if let Some(surface) = envelope.recommended_surface.as_deref() {
        crate::print_surface_line(crate::RenderMode::Plain, "recommended_surface", surface);
    }
    crate::print_surface_line(crate::RenderMode::Plain, "reason", &envelope.reason);
    if !envelope.blocker_codes.is_empty() {
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "blocker_codes",
            &envelope.blocker_codes.join(", "),
        );
    }
    if envelope.status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}

fn emit_blocked_lane_envelope(as_json: bool) -> ExitCode {
    let latest_command =
        crate::operator_command_text::human_command("vida lane show --latest --json");
    let run_command = crate::operator_command_text::human_command("vida lane show <run-id> --json");
    let next_actions = vec![
        format!(
            "Use `{latest_command}` or `{run_command}` to inspect the current lane envelope, then record exception-path evidence with `vida lane exception-takeover` or explicit supersession with `vida lane supersede` as needed."
        ),
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

fn emit_missing_lane_receipt_envelope(
    as_json: bool,
    run_id: Option<&str>,
    surface: &'static str,
) -> ExitCode {
    let blocker_code = if run_id.is_some() {
        "missing_lane_receipt"
    } else {
        "missing_latest_lane_receipt"
    };
    let reason = run_id.map_or_else(
        || "No latest lane receipt exists for the current session.".to_string(),
        |run_id| format!("Missing lane receipt for `{run_id}`."),
    );
    let next_actions = vec![
        format!(
            "Run `{}` and `{}` to confirm the active bounded unit.",
            crate::operator_command_text::human_command("vida status --json"),
            crate::operator_command_text::human_command("vida task next-lawful --json")
        ),
        "Create or refresh a dispatch packet before inspecting lane takeover readiness."
            .to_string(),
    ];
    let artifact_refs = serde_json::json!({
        "surface": surface,
        "run_id": run_id,
        "receipt_required": true,
    });
    let operator_contracts = render_operator_contract_envelope(
        "blocked",
        vec![blocker_code.to_string()],
        next_actions.clone(),
        artifact_refs,
    );
    let envelope = BlockedLaneEnvelope {
        surface,
        status: "blocked",
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
        blocker_codes: vec![blocker_code.to_string()],
        reason,
    };

    if crate::surface_render::print_surface_json(
        &envelope,
        as_json,
        "missing lane receipt surface should serialize",
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

fn lane_show_projection_name(run_id: &str) -> String {
    let suffix = run_id
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || value == '-' || value == '_' {
                value
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("lane-show-{suffix}")
}

fn write_lane_show_projection_cache(state_dir: &Path, run_id: &str, envelope: &LaneEnvelope) {
    if let Ok(payload) = serde_json::to_value(envelope) {
        crate::operator_projection_cache::write_json_projection(
            state_dir,
            &lane_show_projection_name(run_id),
            &payload,
        );
    }
}

fn emit_cached_lane_show_projection(cached: String) -> ExitCode {
    let status = serde_json::from_str::<serde_json::Value>(&cached)
        .ok()
        .and_then(|value| value["status"].as_str().map(ToOwned::to_owned));
    println!("{cached}");
    if status.as_deref() == Some("pass") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}

fn read_cached_lane_show_projection(state_dir: &Path, projection_name: &str) -> Option<String> {
    crate::operator_projection_cache::read_fresh_json_projection(state_dir, projection_name)
}

fn emit_lane_envelope_with_projection_cache(
    state_dir: &Path,
    run_id: &str,
    envelope: &LaneEnvelope,
    as_json: bool,
) -> ExitCode {
    if as_json {
        write_lane_show_projection_cache(state_dir, run_id, envelope);
    }
    emit_lane_envelope(envelope, as_json)
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
            "Lane `{run_id}` is no longer active for mutation because run-graph status is terminal (`{}` / `{}`). Inspect `{}` for the persisted lane envelope and continuation evidence. {next_action}",
            status.status, status.lifecycle_stage,
            crate::operator_command_text::human_command(&format!("vida lane show {run_id} --json")),
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

fn write_synthetic_stale_run_retire_packet(
    state_root: &Path,
    run_id: &str,
    status: &crate::state_store::RunGraphStatus,
) -> Result<String, String> {
    let packet_dir = state_root
        .join("runtime-consumption")
        .join("dispatch-packets");
    std::fs::create_dir_all(&packet_dir).map_err(|error| {
        format!(
            "Failed to create synthetic stale-run retire packet directory `{}`: {error}",
            packet_dir.display()
        )
    })?;
    let packet_path = packet_dir.join(format!("{run_id}-stale-retire.json"));
    let packet = serde_json::json!({
        "packet_kind": "runtime_stale_run_retire_packet",
        "run_id": run_id,
        "task_id": status.task_id,
        "dispatch_target": status.active_node,
        "reason": "missing_task_stale_run_without_lane_receipt",
    });
    std::fs::write(&packet_path, packet.to_string()).map_err(|error| {
        format!(
            "Failed to write synthetic stale-run retire packet `{}`: {error}",
            packet_path.display()
        )
    })?;
    Ok(packet_path.display().to_string())
}

fn synthetic_missing_task_stale_run_receipt(
    state_root: &Path,
    run_id: &str,
    status: &crate::state_store::RunGraphStatus,
) -> Result<crate::state_store::RunGraphDispatchReceipt, String> {
    let packet_path = write_synthetic_stale_run_retire_packet(state_root, run_id, status)?;
    Ok(crate::state_store::RunGraphDispatchReceipt {
        run_id: run_id.to_string(),
        dispatch_target: status.active_node.clone(),
        dispatch_status: "blocked".to_string(),
        lane_status: crate::LaneStatus::LaneBlocked.as_str().to_string(),
        supersedes_receipt_id: None,
        exception_path_receipt_id: None,
        dispatch_kind: "stale_run_retire".to_string(),
        dispatch_surface: Some("vida lane retire".to_string()),
        dispatch_command: Some(format!(
            "vida lane retire {} --receipt-id {} --reason {}",
            crate::shell_quote(run_id),
            crate::shell_quote(run_id),
            crate::shell_quote("missing TaskFlow task stale run")
        )),
        dispatch_packet_path: Some(packet_path),
        dispatch_result_path: None,
        blocker_code: Some("stale_missing_task_run_graph".to_string()),
        downstream_dispatch_target: None,
        downstream_dispatch_command: None,
        downstream_dispatch_note: Some(
            "synthetic cleanup receipt for missing TaskFlow task stale run".to_string(),
        ),
        downstream_dispatch_ready: false,
        downstream_dispatch_blockers: vec!["stale_missing_task_run_graph".to_string()],
        downstream_dispatch_packet_path: None,
        downstream_dispatch_status: Some("blocked".to_string()),
        downstream_dispatch_result_path: None,
        downstream_dispatch_trace_path: None,
        downstream_dispatch_executed_count: 0,
        downstream_dispatch_active_target: Some(status.active_node.clone()),
        downstream_dispatch_last_target: None,
        activation_agent_type: None,
        activation_runtime_role: None,
        selected_backend: Some(status.selected_backend.clone()),
        recorded_at: time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
    })
}

pub(crate) fn missing_task_stale_blocked_run_can_retire(
    status: &crate::state_store::RunGraphStatus,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if crate::taskflow_run_graph_task_authority::run_graph_status_is_terminal_closure(status) {
        return false;
    }

    let lane_status = receipt.lane_status.as_str();
    let blocked_or_running = matches!(
        lane_status,
        value if value == crate::LaneStatus::LaneRunning.as_str()
            || value == crate::LaneStatus::LaneBlocked.as_str()
    );
    let prelaunch_packet_ready = receipt.dispatch_status == "executed"
        && lane_status == crate::LaneStatus::LaneCompleted.as_str()
        && receipt.downstream_dispatch_status.as_deref() == Some("packet_ready");

    (receipt.dispatch_status == "blocked" && blocked_or_running) || prelaunch_packet_ready
}

const MAX_LANE_PACKET_READ_BYTES: u64 = 1024 * 1024;

fn read_lane_packet(state_root: &Path, path: &str) -> Result<serde_json::Value, String> {
    let normalized_path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(path);
    let canonical_path =
        canonicalize_existing_regular_state_path(state_root, &normalized_path, "lane packet")?;
    let metadata = std::fs::symlink_metadata(&canonical_path).map_err(|error| {
        format!(
            "Failed to inspect persisted lane packet `{}`: {error}",
            canonical_path.display()
        )
    })?;
    if metadata.len() > MAX_LANE_PACKET_READ_BYTES {
        return Err(format!(
            "Persisted lane packet `{}` exceeds {} bytes.",
            canonical_path.display(),
            MAX_LANE_PACKET_READ_BYTES
        ));
    }
    let raw = std::fs::read(&canonical_path).map_err(|error| {
        format!(
            "Failed to read persisted lane packet `{}`: {error}",
            canonical_path.display()
        )
    })?;
    serde_json::from_slice(&raw).map_err(|error| {
        format!(
            "Failed to decode persisted lane packet `{}`: {error}",
            canonical_path.display()
        )
    })
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

#[derive(Debug)]
struct HostBridgeCompletionEvidence {
    result_path: String,
    receipt_path: String,
    execution_state: String,
    blocker_code: Option<String>,
    blocker_codes: Vec<String>,
}

struct HostBridgeImplementationArtifacts {
    artifacts: serde_json::Value,
    source: &'static str,
    artifact_refs: Vec<String>,
    blocker_codes: Vec<String>,
}

#[derive(Default)]
struct HostBridgeImplementationAuthority {
    task_id: String,
    task_updated_at: String,
}

#[derive(Default)]
struct HostBridgeTaskflowImplementationEvidence {
    authority: Option<HostBridgeImplementationAuthority>,
    taskflow_artifacts: crate::runtime_dispatch_packets::TaskflowImplementationArtifacts,
    blocker_codes: Vec<String>,
}

struct HostBridgeReceiptPaths {
    request_path: PathBuf,
    packet_path: Option<PathBuf>,
    result_path: PathBuf,
    receipt_path: PathBuf,
}

#[derive(Clone)]
struct HostBridgeCompletionRequestContext {
    dispatch_target: String,
    packet_path: String,
}

const MAX_HOST_BRIDGE_REQUEST_BYTES: u64 = 1024 * 1024;

fn read_host_bridge_request_at_path(path: &Path) -> Result<serde_json::Value, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        format!(
            "Failed to inspect host bridge request `{}`: {error}",
            path.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "Host bridge request `{}` is not a regular file.",
            path.display()
        ));
    }
    if metadata.len() > MAX_HOST_BRIDGE_REQUEST_BYTES {
        return Err(format!(
            "Host bridge request `{}` exceeds {} bytes.",
            path.display(),
            MAX_HOST_BRIDGE_REQUEST_BYTES
        ));
    }
    let raw = std::fs::read(path).map_err(|error| {
        format!(
            "Failed to read host bridge request `{}`: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&raw).map_err(|error| {
        format!(
            "Failed to decode host bridge request `{}` as JSON: {error}",
            path.display()
        )
    })
}

fn read_host_bridge_request(state_root: &Path, path: &str) -> Result<serde_json::Value, String> {
    let normalized_path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(path);
    let canonical_path = canonicalize_existing_regular_state_path(
        state_root,
        Path::new(&normalized_path),
        "request",
    )?;
    read_host_bridge_request_at_path(&canonical_path)
}

fn host_bridge_path_string<'a>(
    request: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, String> {
    request
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Host bridge request is missing non-empty `{field}`."))
}

fn host_bridge_packet_confirms_active_request(
    packet_path: &Path,
    run_id: &str,
    dispatch_target: &str,
) -> bool {
    let Ok(packet) = read_host_bridge_request_at_path(packet_path) else {
        return false;
    };
    if packet.get("run_id").and_then(serde_json::Value::as_str) != Some(run_id) {
        return false;
    }
    let direct_target = packet
        .get("dispatch_target")
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    let downstream_active_target = packet
        .get("downstream_dispatch_active_target")
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    let downstream_status = packet
        .get("downstream_dispatch_status")
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    (direct_target == Some(dispatch_target)
        && downstream_active_target == Some(dispatch_target)
        && downstream_status == Some("blocked"))
        || (direct_target == Some(dispatch_target) && downstream_status.is_none())
}

fn host_bridge_request_is_retryable_completion_state(request: &serde_json::Value) -> bool {
    matches!(
        request
            .get("status")
            .and_then(serde_json::Value::as_str)
            .map(str::trim),
        Some("retryable_blocked")
    )
}

fn trusted_host_bridge_completion_request_context(
    state_root: &Path,
    run_id: &str,
    request_path: &str,
    status: Option<&crate::state_store::RunGraphStatus>,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<HostBridgeCompletionRequestContext>, String> {
    let Some(status) = status else {
        return Ok(None);
    };
    let request = read_host_bridge_request(state_root, request_path)?;
    if request.get("run_id").and_then(serde_json::Value::as_str) != Some(run_id) {
        return Ok(None);
    }
    if request
        .get("dispatch_transport")
        .and_then(serde_json::Value::as_str)
        != Some("host_tool_bridge")
    {
        return Ok(None);
    }
    let dispatch_target = host_bridge_path_string(&request, "dispatch_target")?;
    if status.active_node.trim() != dispatch_target {
        return Ok(None);
    }
    let adapter_gate_context =
        status.status == "blocked" && status.policy_gate == "host_tool_bridge_adapter_required";
    let retryable_completion_context = status.status == "blocked"
        && (host_bridge_completion_request_required(receipt)
            || host_bridge_request_is_retryable_completion_state(&request)
            || host_bridge_request_has_retryable_completion_evidence(state_root, request_path));
    if !adapter_gate_context && !retryable_completion_context {
        return Ok(None);
    }
    let packet_path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(
        host_bridge_path_string(&request, "packet_path")?,
    );
    let packet_path = canonicalize_existing_regular_state_path(state_root, &packet_path, "packet")?;
    if let Some(selected_backend) = receipt.selected_backend.as_deref() {
        let backend_id = host_bridge_path_string(&request, "backend_id")?;
        if backend_id != selected_backend {
            return Err(
                "Host bridge request backend does not match persisted dispatch receipt evidence."
                    .to_string(),
            );
        }
    }
    let receipt_target_matches_request = receipt.dispatch_target.trim() == dispatch_target;
    if retryable_completion_context && !receipt_target_matches_request {
        return Err(
            "Retryable host bridge request dispatch target does not match persisted dispatch receipt evidence."
                .to_string(),
        );
    }
    if !receipt_target_matches_request
        && !host_bridge_packet_confirms_active_request(&packet_path, run_id, dispatch_target)
    {
        return Err(
            "Host bridge request dispatch target does not match persisted dispatch receipt evidence."
                .to_string(),
        );
    }
    if receipt_target_matches_request {
        if let Some(authoritative_packet_path) = receipt.downstream_dispatch_packet_path.as_deref()
        {
            let authoritative_packet_path =
                crate::runtime_dispatch_state::normalize_persisted_runtime_path(
                    authoritative_packet_path,
                );
            let authoritative_packet_path =
                canonicalize_existing_state_path(state_root, &authoritative_packet_path, "packet")?;
            if packet_path != authoritative_packet_path {
                return Err(
                "Host bridge request packet path does not match persisted dispatch receipt evidence."
                    .to_string(),
            );
            }
        } else if retryable_completion_context
            && receipt.dispatch_status != "bridge_request_pending"
        {
            return Err(
                "Retryable host bridge request is missing persisted dispatch packet evidence."
                    .to_string(),
            );
        }
    }
    Ok(Some(HostBridgeCompletionRequestContext {
        dispatch_target: dispatch_target.to_string(),
        packet_path: packet_path.display().to_string(),
    }))
}

fn path_has_dot_segment(path: &Path) -> bool {
    path.as_os_str()
        .to_string_lossy()
        .split(['/', '\\'])
        .any(|segment| matches!(segment, "." | ".."))
}

fn canonical_state_root(state_root: &Path) -> Result<PathBuf, String> {
    std::fs::canonicalize(state_root).map_err(|error| {
        format!(
            "Failed to canonicalize VIDA state root `{}`: {error}",
            state_root.display()
        )
    })
}

fn canonicalize_existing_state_path(
    state_root: &Path,
    path: &Path,
    label: &str,
) -> Result<PathBuf, String> {
    if path_has_dot_segment(path) {
        return Err(format!(
            "Host bridge {label} path `{}` contains inadmissible dot-segment traversal.",
            path.display()
        ));
    }
    let canonical_path = std::fs::canonicalize(path).map_err(|error| {
        format!(
            "Failed to canonicalize host bridge {label} path `{}`: {error}",
            path.display()
        )
    })?;
    let canonical_root = canonical_state_root(state_root)?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(format!(
            "Host bridge {label} path `{}` is outside VIDA state root `{}`.",
            canonical_path.display(),
            canonical_root.display()
        ));
    }
    Ok(canonical_path)
}

fn canonicalize_existing_regular_state_path(
    state_root: &Path,
    path: &Path,
    label: &str,
) -> Result<PathBuf, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        format!(
            "Failed to inspect host bridge {label} path `{}`: {error}",
            path.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "Host bridge {label} path `{}` is not a regular file.",
            path.display()
        ));
    }
    canonicalize_existing_state_path(state_root, path, label)
}

fn validate_new_state_artifact_path(
    state_root: &Path,
    path: &Path,
    label: &str,
) -> Result<PathBuf, String> {
    if path_has_dot_segment(path) {
        return Err(format!(
            "Host bridge {label} path `{}` contains inadmissible dot-segment traversal.",
            path.display()
        ));
    }
    if std::fs::symlink_metadata(path).is_ok() {
        return Err(format!(
            "Host bridge {label} path `{}` already exists; refusing to overwrite it.",
            path.display()
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        format!(
            "Host bridge {label} path `{}` has no parent directory.",
            path.display()
        )
    })?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create host bridge {label} directory: {error}"))?;
    let canonical_parent = std::fs::canonicalize(parent).map_err(|error| {
        format!(
            "Failed to canonicalize host bridge {label} directory `{}`: {error}",
            parent.display()
        )
    })?;
    let canonical_root = canonical_state_root(state_root)?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(format!(
            "Host bridge {label} directory `{}` is outside VIDA state root `{}`.",
            canonical_parent.display(),
            canonical_root.display()
        ));
    }
    let _file_name = path.file_name().ok_or_else(|| {
        format!(
            "Host bridge {label} path `{}` has no file name.",
            path.display()
        )
    })?;
    Ok(path.to_path_buf())
}

fn validate_state_artifact_path_for_host_bridge_write(
    state_root: &Path,
    path: &Path,
    label: &str,
    replace_existing: bool,
) -> Result<PathBuf, String> {
    if replace_existing && std::fs::symlink_metadata(path).is_ok() {
        return canonicalize_existing_state_path(state_root, path, label);
    }
    validate_new_state_artifact_path(state_root, path, label)
}

fn host_bridge_request_object(result: &serde_json::Value) -> Option<&serde_json::Value> {
    result.get("host_tool_bridge_request").or_else(|| {
        result
            .get("backend_dispatch")
            .and_then(|dispatch| dispatch.get("host_tool_bridge_request"))
    })
}

fn host_bridge_request_paths_from_dispatch_result(
    result: &serde_json::Value,
) -> Result<HostBridgeReceiptPaths, String> {
    let request = host_bridge_request_object(result).ok_or_else(|| {
        "Persisted host bridge dispatch result is missing `host_tool_bridge_request`.".to_string()
    })?;
    host_bridge_request_paths_from_request_object(request)
}

fn host_bridge_request_paths_from_request_object(
    request: &serde_json::Value,
) -> Result<HostBridgeReceiptPaths, String> {
    Ok(HostBridgeReceiptPaths {
        request_path: crate::runtime_dispatch_state::normalize_persisted_runtime_path(
            host_bridge_path_string(request, "request_path")?,
        ),
        packet_path: request
            .get("packet_path")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(crate::runtime_dispatch_state::normalize_persisted_runtime_path),
        result_path: crate::runtime_dispatch_state::normalize_persisted_runtime_path(
            host_bridge_path_string(request, "result_path")?,
        ),
        receipt_path: crate::runtime_dispatch_state::normalize_persisted_runtime_path(
            host_bridge_path_string(request, "receipt_path")?,
        ),
    })
}

fn validated_host_bridge_paths_from_receipt(
    state_root: &Path,
    request_path: &Path,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    replace_existing_evidence: bool,
    allow_reconciled_request_paths: bool,
) -> Result<HostBridgeReceiptPaths, String> {
    let canonical_request_path =
        canonicalize_existing_state_path(state_root, request_path, "request")?;
    let Some(dispatch_result_path) = receipt.dispatch_result_path.as_deref() else {
        return Err(
            "Lane receipt is missing persisted host bridge dispatch result evidence.".into(),
        );
    };
    let dispatch_result_path =
        crate::runtime_dispatch_state::normalize_persisted_runtime_path(dispatch_result_path);
    let dispatch_result_path = canonicalize_existing_regular_state_path(
        state_root,
        &dispatch_result_path,
        "dispatch result",
    )?;
    let result = read_host_bridge_request_at_path(&dispatch_result_path)?;
    let paths = match host_bridge_request_paths_from_dispatch_result(&result) {
        Ok(paths) => paths,
        Err(error) if allow_reconciled_request_paths => {
            let mut request = read_host_bridge_request_at_path(&canonical_request_path)?;
            if request
                .get("request_path")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
            {
                if let Some(object) = request.as_object_mut() {
                    object.insert(
                        "request_path".to_string(),
                        serde_json::json!(canonical_request_path.display().to_string()),
                    );
                }
            }
            host_bridge_request_paths_from_request_object(&request).map_err(|request_error| {
                format!("{error}; reconciled request path evidence is invalid: {request_error}")
            })?
        }
        Err(error) => return Err(error),
    };
    let canonical_paths_request_path =
        canonicalize_existing_regular_state_path(state_root, &paths.request_path, "request")?;
    if canonical_request_path != canonical_paths_request_path {
        return Err(format!(
            "Host bridge request `{}` does not match persisted dispatch receipt request `{}`.",
            request_path.display(),
            canonical_paths_request_path.display()
        ));
    }
    let request = read_host_bridge_request_at_path(&canonical_request_path)?;
    let request_result_path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(
        host_bridge_path_string(&request, "result_path")?,
    );
    let request_receipt_path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(
        host_bridge_path_string(&request, "receipt_path")?,
    );
    if request_result_path != paths.result_path || request_receipt_path != paths.receipt_path {
        return Err(
            "Host bridge request result/receipt paths do not match persisted dispatch receipt evidence."
                .to_string(),
        );
    }
    if let Some(authoritative_packet_path) = paths.packet_path.as_ref() {
        let request_packet_path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(
            host_bridge_path_string(&request, "packet_path")?,
        );
        let request_packet_path =
            canonicalize_existing_regular_state_path(state_root, &request_packet_path, "packet")?;
        let authoritative_packet_path = canonicalize_existing_regular_state_path(
            state_root,
            authoritative_packet_path,
            "packet",
        )?;
        if request_packet_path != authoritative_packet_path {
            return Err(
                "Host bridge request packet path does not match persisted dispatch receipt evidence."
                    .to_string(),
            );
        }
    }
    Ok(HostBridgeReceiptPaths {
        request_path: canonical_request_path,
        packet_path: paths.packet_path,
        result_path: validate_state_artifact_path_for_host_bridge_write(
            state_root,
            &paths.result_path,
            "result",
            replace_existing_evidence,
        )?,
        receipt_path: validate_state_artifact_path_for_host_bridge_write(
            state_root,
            &paths.receipt_path,
            "receipt",
            replace_existing_evidence,
        )?,
    })
}

fn write_json_artifact_new(
    path: &Path,
    value: &serde_json::Value,
    label: &str,
) -> Result<(), String> {
    let encoded = serde_json::to_string_pretty(value)
        .map_err(|error| format!("Failed to encode {label}: {error}"))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("Failed to create {label} `{}`: {error}", path.display()))?;
    use std::io::Write;
    file.write_all(encoded.as_bytes())
        .map_err(|error| format!("Failed to write {label} `{}`: {error}", path.display()))
}

fn write_json_artifact_replace_existing(
    path: &Path,
    value: &serde_json::Value,
    label: &str,
) -> Result<(), String> {
    let encoded = serde_json::to_string_pretty(value)
        .map_err(|error| format!("Failed to encode {label}: {error}"))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|error| format!("Failed to open {label} `{}`: {error}", path.display()))?;
    use std::io::Write;
    file.write_all(encoded.as_bytes())
        .map_err(|error| format!("Failed to write {label} `{}`: {error}", path.display()))
}

fn host_bridge_implementation_artifacts(
    request: &serde_json::Value,
    taskflow_artifacts: crate::runtime_dispatch_packets::TaskflowImplementationArtifacts,
    completion_receipt_id: &str,
    task_authority: Option<&HostBridgeImplementationAuthority>,
) -> HostBridgeImplementationArtifacts {
    if !taskflow_artifacts.artifacts.is_empty() {
        return HostBridgeImplementationArtifacts {
            artifacts: serde_json::Value::Array(taskflow_artifacts.artifacts),
            source: "taskflow_attempt_ledger",
            artifact_refs: taskflow_artifacts.artifact_refs,
            blocker_codes: Vec::new(),
        };
    }
    let request_artifacts = request
        .get("implementation_artifacts")
        .filter(|value| value.as_array().is_some_and(|rows| !rows.is_empty()));
    if let Some(request_artifacts) = request_artifacts {
        let blocker_codes = if host_bridge_request_artifacts_are_taskflow_authorized(
            request_artifacts,
            &taskflow_artifacts.authority_keys,
        ) {
            Vec::new()
        } else if let Some(authority) = task_authority {
            if host_bridge_request_artifacts_are_bare_completion_candidates(request_artifacts) {
                return HostBridgeImplementationArtifacts {
                    artifacts: host_bridge_completion_authorized_request_artifacts(
                        request_artifacts,
                        authority,
                        completion_receipt_id,
                    ),
                    source: "host_bridge_completion_receipt",
                    artifact_refs: Vec::new(),
                    blocker_codes: Vec::new(),
                };
            }
            vec!["implementation_artifact_receipt_unverified".to_string()]
        } else {
            vec!["implementation_artifact_receipt_unverified".to_string()]
        };
        return HostBridgeImplementationArtifacts {
            artifacts: request_artifacts.clone(),
            source: "host_bridge_request",
            artifact_refs: Vec::new(),
            blocker_codes,
        };
    }
    HostBridgeImplementationArtifacts {
        artifacts: request
            .get("implementation_artifacts")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        source: "host_bridge_request",
        artifact_refs: Vec::new(),
        blocker_codes: Vec::new(),
    }
}

fn host_bridge_request_artifacts_are_bare_completion_candidates(
    request_artifacts: &serde_json::Value,
) -> bool {
    let Some(rows) = request_artifacts.as_array() else {
        return false;
    };
    !rows.is_empty()
        && rows.iter().all(|artifact| {
            let Some(object) = artifact.as_object() else {
                return false;
            };
            let receipt_backed = object
                .get("receipt_backed")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let freshness = object
                .get("freshness")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let consolidation_receipt_id = object
                .get("consolidation_receipt_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            !receipt_backed && freshness.is_none() && consolidation_receipt_id.is_none()
        })
}

fn host_bridge_completion_authorized_request_artifacts(
    request_artifacts: &serde_json::Value,
    authority: &HostBridgeImplementationAuthority,
    completion_receipt_id: &str,
) -> serde_json::Value {
    let mut artifacts = request_artifacts.clone();
    if let Some(rows) = artifacts.as_array_mut() {
        for artifact in rows.iter_mut() {
            if let Some(object) = artifact.as_object_mut() {
                object.insert(
                    "freshness".to_string(),
                    serde_json::json!(authority.task_updated_at),
                );
                object.insert("receipt_backed".to_string(), serde_json::json!(true));
                object.insert(
                    "consolidation_receipt_id".to_string(),
                    serde_json::json!(completion_receipt_id),
                );
            }
        }
    }
    artifacts
}

fn host_bridge_request_artifacts_are_taskflow_authorized(
    request_artifacts: &serde_json::Value,
    authority_keys: &[crate::runtime_dispatch_packets::TaskflowImplementationArtifactAuthority],
) -> bool {
    let Some(rows) = request_artifacts.as_array() else {
        return false;
    };
    !rows.is_empty()
        && rows.iter().all(|artifact| {
            let Some(object) = artifact.as_object() else {
                return false;
            };
            let attempt_id = object
                .get("attempt_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim);
            let task_id = object
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim);
            let stage_id = object
                .get("stage_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim);
            let freshness = object
                .get("freshness")
                .and_then(serde_json::Value::as_str)
                .map(str::trim);
            let consolidation_receipt_id = object
                .get("consolidation_receipt_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim);
            authority_keys.iter().any(|authority| {
                attempt_id == Some(authority.attempt_id.as_str())
                    && task_id == Some(authority.task_id.as_str())
                    && stage_id == Some(authority.stage_id.as_str())
                    && freshness == Some(authority.freshness.as_str())
                    && consolidation_receipt_id == Some(authority.consolidation_receipt_id.as_str())
            })
        })
}

fn host_bridge_implementation_scope_validation(
    request: &serde_json::Value,
    artifacts: &serde_json::Value,
    authority: crate::runtime_dispatch_packets::ImplementationArtifactAuthority<'_>,
    authoritative_owned_paths: &[String],
) -> serde_json::Value {
    let isolation = request.get("implementation_isolation");
    let isolation_is_valid = isolation.is_some_and(|value| value.is_object());
    let mut validation = crate::runtime_dispatch_packets::implementation_artifact_scope_validation(
        authoritative_owned_paths,
        artifacts,
        authority,
    );
    if !isolation_is_valid {
        let mut blocker_codes = host_bridge_scope_validation_blocker_codes(&validation);
        blocker_codes.push("implementation_artifact_contract_invalid".to_string());
        blocker_codes.sort();
        blocker_codes.dedup();
        if let Some(object) = validation.as_object_mut() {
            object.insert("status".to_string(), serde_json::json!("blocked"));
            object.insert(
                "blocker_codes".to_string(),
                serde_json::json!(blocker_codes),
            );
        }
    }
    validation
}

fn owned_paths_from_lane_packet(packet: &serde_json::Value) -> Vec<String> {
    let packet_kind = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    packet_kind
        .and_then(|kind| packet.get(kind).and_then(|body| body.get("owned_paths")))
        .and_then(serde_json::Value::as_array)
        .and_then(|paths| {
            paths
                .iter()
                .map(|path| {
                    path.as_str()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                })
                .collect::<Option<Vec<_>>>()
        })
        .unwrap_or_default()
}

fn host_bridge_scope_validation_blocker_codes(validation: &serde_json::Value) -> Vec<String> {
    validation
        .get("blocker_codes")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>()
}

fn host_bridge_completion_requires_implementation_artifacts(dispatch_target: &str) -> bool {
    matches!(dispatch_target.trim(), "implementer" | "implementation")
}

fn host_bridge_completion_summary_blocker_code(
    dispatch_target: &str,
    summary: &str,
) -> Option<String> {
    crate::runtime_dispatch_state::runtime_lane_completion_summary_blocker_code(
        dispatch_target,
        Some(summary),
    )
    .or_else(|| {
        let normalized = summary.trim().to_ascii_lowercase();
        if normalized.contains("blocked by ") {
            Some("lane_completion_blocked_by_summary".to_string())
        } else {
            None
        }
    })
}

async fn taskflow_implementation_artifacts_for_host_bridge_request(
    store: &StateStore,
    request_path: &str,
    run_id: &str,
) -> HostBridgeTaskflowImplementationEvidence {
    let request = match read_host_bridge_request(store.root(), request_path) {
        Ok(request) => request,
        Err(_) => return HostBridgeTaskflowImplementationEvidence::default(),
    };
    if request
        .get("task_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some_and(|task_id| task_id != run_id)
    {
        return HostBridgeTaskflowImplementationEvidence {
            authority: None,
            taskflow_artifacts:
                crate::runtime_dispatch_packets::TaskflowImplementationArtifacts::default(),
            blocker_codes: vec!["host_bridge_request_task_mismatch".to_string()],
        };
    }
    let task = match store.show_task(run_id).await {
        Ok(task) => task,
        Err(_) => {
            return HostBridgeTaskflowImplementationEvidence {
                authority: None,
                taskflow_artifacts:
                    crate::runtime_dispatch_packets::TaskflowImplementationArtifacts::default(),
                blocker_codes: vec!["implementation_artifact_authority_missing".to_string()],
            }
        }
    };
    let authority = HostBridgeImplementationAuthority {
        task_id: task.id.clone(),
        task_updated_at: task.updated_at.clone(),
    };
    let taskflow_artifacts = match store.task_stage_attempts(run_id, "implementation").await {
        Ok(attempts) => {
            match crate::runtime_dispatch_packets::taskflow_attempt_implementation_artifacts(
                &attempts,
                &task.updated_at,
                store.root(),
            ) {
                Ok(artifacts) => artifacts,
                Err(_) => return HostBridgeTaskflowImplementationEvidence {
                    authority: Some(authority),
                    taskflow_artifacts:
                        crate::runtime_dispatch_packets::TaskflowImplementationArtifacts::default(),
                    blocker_codes: vec!["implementation_artifact_contract_invalid".to_string()],
                },
            }
        }
        Err(_) => crate::runtime_dispatch_packets::TaskflowImplementationArtifacts::default(),
    };
    HostBridgeTaskflowImplementationEvidence {
        authority: Some(authority),
        taskflow_artifacts,
        blocker_codes: Vec::new(),
    }
}

fn host_bridge_completion_retryable_blocker(blocker_code: &str) -> bool {
    crate::runtime_dispatch_packets::host_bridge_completion_retryable_blocker(blocker_code)
}

fn host_bridge_artifact_has_retryable_completion_blocker(artifact: &serde_json::Value) -> bool {
    artifact
        .get("blocker_code")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .is_some_and(host_bridge_completion_retryable_blocker)
        || artifact
            .get("blocker_codes")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|blockers| {
                blockers.iter().any(|blocker| {
                    blocker
                        .as_str()
                        .map(str::trim)
                        .is_some_and(host_bridge_completion_retryable_blocker)
                })
            })
}

fn host_bridge_request_has_retryable_completion_evidence(
    state_root: &Path,
    request_path: &str,
) -> bool {
    let Ok(request) = read_host_bridge_request(state_root, request_path) else {
        return false;
    };
    for field in ["receipt_path", "result_path"] {
        let Ok(raw_path) = host_bridge_path_string(&request, field) else {
            continue;
        };
        let path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(raw_path);
        let Ok(path) = canonicalize_existing_regular_state_path(state_root, &path, field) else {
            continue;
        };
        let Ok(artifact) = read_host_bridge_request_at_path(&path) else {
            continue;
        };
        if artifact.get("status").and_then(serde_json::Value::as_str) == Some("blocked")
            && host_bridge_artifact_has_retryable_completion_blocker(&artifact)
        {
            return true;
        }
    }
    false
}

fn host_bridge_completion_request_required(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    receipt.dispatch_status == "bridge_request_pending"
        || (receipt.dispatch_status == "blocked"
            && (receipt
                .blocker_code
                .as_deref()
                .is_some_and(host_bridge_completion_retryable_blocker)
                || receipt
                    .downstream_dispatch_blockers
                    .iter()
                    .any(|blocker| host_bridge_completion_retryable_blocker(blocker))))
}

fn materialize_host_bridge_completion_evidence(
    state_root: &Path,
    request_path: &str,
    run_id: &str,
    dispatch_target: &str,
    persisted_receipt: &crate::state_store::RunGraphDispatchReceipt,
    receipt_id: &str,
    host_agent_id: Option<&str>,
    summary: Option<&str>,
    taskflow_evidence: HostBridgeTaskflowImplementationEvidence,
    authoritative_owned_paths: &[String],
    replace_existing_evidence: bool,
    allow_reconciled_request_paths: bool,
) -> Result<HostBridgeCompletionEvidence, String> {
    let normalized_request_path =
        crate::runtime_dispatch_state::normalize_persisted_runtime_path(request_path);
    let canonical_request_path =
        canonicalize_existing_state_path(state_root, &normalized_request_path, "request")?;
    let validated_paths = validated_host_bridge_paths_from_receipt(
        state_root,
        &canonical_request_path,
        persisted_receipt,
        replace_existing_evidence,
        allow_reconciled_request_paths,
    )?;
    let mut request = read_host_bridge_request_at_path(&canonical_request_path)?;
    if request.get("run_id").and_then(serde_json::Value::as_str) != Some(run_id) {
        return Err(format!(
            "Host bridge request `{request_path}` does not belong to run `{run_id}`."
        ));
    }
    if request
        .get("dispatch_target")
        .and_then(serde_json::Value::as_str)
        != Some(dispatch_target)
    {
        return Err(format!(
            "Host bridge request `{request_path}` does not belong to dispatch target `{dispatch_target}`."
        ));
    }
    if request
        .get("dispatch_transport")
        .and_then(serde_json::Value::as_str)
        != Some("host_tool_bridge")
    {
        return Err(format!(
            "Host bridge request `{request_path}` is not a `host_tool_bridge` request."
        ));
    }
    let request_result_path = validate_state_artifact_path_for_host_bridge_write(
        state_root,
        &crate::runtime_dispatch_state::normalize_persisted_runtime_path(host_bridge_path_string(
            &request,
            "result_path",
        )?),
        "result",
        replace_existing_evidence,
    )?;
    let request_receipt_path = validate_state_artifact_path_for_host_bridge_write(
        state_root,
        &crate::runtime_dispatch_state::normalize_persisted_runtime_path(host_bridge_path_string(
            &request,
            "receipt_path",
        )?),
        "receipt",
        replace_existing_evidence,
    )?;
    if request_result_path != validated_paths.result_path
        || request_receipt_path != validated_paths.receipt_path
    {
        return Err(
            "Host bridge request artifact paths do not match persisted dispatch receipt evidence."
                .to_string(),
        );
    }
    let result_path = validated_paths.result_path;
    let receipt_path = validated_paths.receipt_path;
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let request_id = request
        .get("request_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("host-tool-bridge-request");
    let packet_path = request
        .get("packet_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let backend_id = request
        .get("backend_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("internal_subagents");
    let summary = summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("parent host bridge reported internal agent completion");
    let summary_blocker_code =
        host_bridge_completion_summary_blocker_code(dispatch_target, summary);
    let implementation_artifacts = host_bridge_implementation_artifacts(
        &request,
        taskflow_evidence.taskflow_artifacts,
        receipt_id,
        taskflow_evidence.authority.as_ref(),
    );
    let missing_authority_updated_at = "__missing_task_authority__";
    let authority = taskflow_evidence.authority.as_ref().map_or(
        crate::runtime_dispatch_packets::ImplementationArtifactAuthority {
            task_id: run_id,
            task_updated_at: missing_authority_updated_at,
        },
        |authority| crate::runtime_dispatch_packets::ImplementationArtifactAuthority {
            task_id: authority.task_id.as_str(),
            task_updated_at: authority.task_updated_at.as_str(),
        },
    );
    let requires_implementation_artifacts =
        host_bridge_completion_requires_implementation_artifacts(dispatch_target);
    let implementation_scope_validation = if requires_implementation_artifacts {
        host_bridge_implementation_scope_validation(
            &request,
            &implementation_artifacts.artifacts,
            authority,
            authoritative_owned_paths,
        )
    } else {
        serde_json::Value::Null
    };
    let mut blocker_codes = Vec::new();
    if requires_implementation_artifacts {
        blocker_codes.extend(taskflow_evidence.blocker_codes);
        blocker_codes.extend(implementation_artifacts.blocker_codes.clone());
    }
    if implementation_scope_validation
        .get("status")
        .and_then(serde_json::Value::as_str)
        == Some("blocked")
    {
        blocker_codes.extend(host_bridge_scope_validation_blocker_codes(
            &implementation_scope_validation,
        ));
    }
    if let Some(summary_blocker_code) = summary_blocker_code {
        blocker_codes.push(summary_blocker_code);
    }
    blocker_codes.sort();
    blocker_codes.dedup();
    let blocker_code = blocker_codes.first().cloned();
    let execution_state = if !blocker_codes.is_empty() {
        "blocked"
    } else {
        "executed"
    };
    let status = if !blocker_codes.is_empty() {
        "blocked"
    } else {
        "pass"
    };
    let result = serde_json::json!({
        "artifact_kind": "host_tool_bridge_result",
        "schema_version": 1,
        "status": status,
        "execution_state": execution_state,
        "request_id": request_id,
        "run_id": run_id,
        "dispatch_target": dispatch_target,
        "completion_receipt_id": receipt_id,
        "blocker_code": blocker_code.clone(),
        "blocker_codes": blocker_codes.clone(),
        "host_agent_id": host_agent_id,
        "summary": summary,
        "implementation_artifacts": implementation_artifacts.artifacts.clone(),
        "implementation_artifact_source": implementation_artifacts.source,
        "implementation_artifact_refs": implementation_artifacts.artifact_refs.clone(),
        "scope_validation": implementation_scope_validation.clone(),
        "source_dispatch_packet_path": packet_path,
        "host_tool_bridge_request": {
            "request_path": canonical_request_path.display().to_string(),
            "packet_path": packet_path,
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string(),
            "backend_id": backend_id,
            "request_id": request_id,
            "run_id": run_id,
            "dispatch_target": dispatch_target,
            "dispatch_transport": "host_tool_bridge"
        },
        "activation_semantics": {
            "activation_kind": "execution_evidence",
            "view_only": false,
            "executes_packet": true,
            "records_completion_receipt": true,
            "transfers_root_session_write_authority": false,
            "root_session_write_guard_remains_authoritative": true
        },
        "execution_evidence": {
            "status": "recorded",
            "evidence_kind": "host_tool_bridge_result",
            "backend_id": backend_id,
            "receipt_backed": true,
            "completion_verdict": if !blocker_codes.is_empty() { "rework_required" } else { "pass" },
            "records_dispatch_result": true
        },
        "recorded_at": recorded_at,
    });
    let receipt = serde_json::json!({
        "artifact_kind": "host_tool_bridge_receipt",
        "schema_version": 1,
        "status": status,
        "receipt_backed": true,
        "request_id": request_id,
        "run_id": run_id,
        "dispatch_target": dispatch_target,
        "completion_receipt_id": receipt_id,
        "blocker_code": blocker_code.clone(),
        "blocker_codes": blocker_codes.clone(),
        "host_agent_id": host_agent_id,
        "request_path": canonical_request_path.display().to_string(),
        "result_path": result_path.display().to_string(),
        "source_dispatch_packet_path": packet_path,
        "implementation_artifact_source": implementation_artifacts.source,
        "implementation_artifact_refs": implementation_artifacts.artifact_refs,
        "scope_validation": implementation_scope_validation.clone(),
        "recorded_at": recorded_at,
    });
    if replace_existing_evidence && result_path.exists() {
        write_json_artifact_replace_existing(&result_path, &result, "host bridge result")?;
    } else {
        write_json_artifact_new(&result_path, &result, "host bridge result")?;
    }
    if replace_existing_evidence && receipt_path.exists() {
        write_json_artifact_replace_existing(&receipt_path, &receipt, "host bridge receipt")?;
    } else {
        write_json_artifact_new(&receipt_path, &receipt, "host bridge receipt")?;
    }
    let request_status = if status == "blocked"
        && blocker_codes
            .iter()
            .all(|blocker| host_bridge_completion_retryable_blocker(blocker))
    {
        "retryable_blocked"
    } else {
        status
    };
    if let Some(object) = request.as_object_mut() {
        object.insert("status".to_string(), serde_json::json!(request_status));
        object.insert(
            "completion_receipt_id".to_string(),
            serde_json::json!(receipt_id),
        );
        object.insert("completed_at".to_string(), serde_json::json!(recorded_at));
        object.insert(
            "scope_validation".to_string(),
            implementation_scope_validation,
        );
        object.insert(
            "implementation_artifact_source".to_string(),
            serde_json::json!(implementation_artifacts.source),
        );
        object.insert(
            "host_agent_id".to_string(),
            host_agent_id
                .map(|value| serde_json::Value::String(value.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
    }
    write_json_artifact_replace_existing(&canonical_request_path, &request, "host bridge request")?;
    Ok(HostBridgeCompletionEvidence {
        result_path: result_path.display().to_string(),
        receipt_path: receipt_path.display().to_string(),
        execution_state: execution_state.to_string(),
        blocker_code: blocker_code.clone(),
        blocker_codes,
    })
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
    if lane_help_requested(&args.args) {
        println!("{}", lane_help_text(&args.args));
        return ExitCode::SUCCESS;
    }
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
    let state_dir = match &command {
        LaneCommand::Complete { state_dir, .. } => {
            state_dir.map(PathBuf::from).unwrap_or_else(proxy_state_dir)
        }
        _ => proxy_state_dir(),
    };

    match &command {
        LaneCommand::ShowLatest { as_json } => {
            if *as_json {
                if let Some(cached) = read_cached_lane_show_projection(
                    &state_dir,
                    &lane_show_projection_name("latest"),
                ) {
                    return emit_cached_lane_show_projection(cached);
                }
            }
            let store = match StateStore::open_existing_read_only_with_timeout(
                state_dir.clone(),
                LANE_SURFACE_LOCK_TIMEOUT,
            )
            .await
            {
                Ok(store) => store,
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    return ExitCode::from(1);
                }
            };
            let operator_session_projection =
                match crate::operator_session_projection::build_operator_session_projection(&store)
                    .await
                {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("Failed to build operator session projection: {error}");
                        return ExitCode::from(1);
                    }
                };
            let Some(summary) = (match store
                .latest_run_graph_dispatch_receipt_summary_for_current_session()
                .await
            {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("Failed to read latest lane receipt summary: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                return emit_missing_lane_receipt_envelope(*as_json, None, "vida lane show");
            };
            let status = match store.run_graph_status(&summary.run_id).await {
                Ok(status) => Some(status),
                Err(_) => None,
            };
            let retired_closed_task_status =
                retired_closed_task_status_for_show(&store, status.as_ref()).await;
            let closed_task_retired = retired_closed_task_status.is_some();
            let status = retired_closed_task_status.or(status);
            let recovery = status.as_ref().map(|status| {
                crate::state_store::RunGraphRecoverySummary::from_status(status.clone())
            });
            let owned_write_scope_hint =
                task_owned_write_scope_for_status(&store, status.as_ref()).await;
            let summary = if closed_task_retired {
                retired_closed_task_summary_for_show(summary)
            } else {
                summary
            };
            let exception_path_metadata_path = if closed_task_retired {
                None
            } else {
                match exception_takeover_metadata_path(store.root(), &summary.run_id) {
                    Ok(path) => path.exists().then(|| path.display().to_string()),
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                }
            };
            let exception_path_metadata = if closed_task_retired {
                None
            } else {
                match read_exception_takeover_metadata(store.root(), &summary.run_id) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                }
            };
            let truth = derive_lane_show_truth_with_exception_metadata(
                &summary,
                recovery.as_ref(),
                exception_path_metadata.as_ref(),
            );
            let envelope = build_lane_envelope_with_owned_scope(
                summary,
                status,
                exception_path_metadata_path,
                exception_path_metadata,
                operator_session_projection,
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
                &owned_write_scope_hint,
            );
            return emit_lane_envelope_with_projection_cache(
                &state_dir, "latest", &envelope, *as_json,
            );
        }
        LaneCommand::ShowRun { run_id, as_json } => {
            if *as_json {
                if let Some(cached) =
                    read_cached_lane_show_projection(&state_dir, &lane_show_projection_name(run_id))
                {
                    return emit_cached_lane_show_projection(cached);
                }
            }
            let store = match StateStore::open_existing_read_only_with_timeout(
                state_dir.clone(),
                LANE_SURFACE_LOCK_TIMEOUT,
            )
            .await
            {
                Ok(store) => store,
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    return ExitCode::from(1);
                }
            };
            let operator_session_projection =
                match crate::operator_session_projection::build_operator_session_projection(&store)
                    .await
                {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("Failed to build operator session projection: {error}");
                        return ExitCode::from(1);
                    }
                };
            let Some(receipt) = (match store
                .run_graph_dispatch_receipt_for_status(run_id, None)
                .await
            {
                Ok(receipt) => receipt,
                Err(error) => {
                    eprintln!("Failed to read lane receipt `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                return emit_missing_lane_receipt_envelope(
                    *as_json,
                    Some(run_id),
                    "vida lane show",
                );
            };
            let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let needs_status_projection = summary.lane_status
                != crate::LaneStatus::LaneCompleted.as_str()
                || summary.dispatch_status != "executed"
                || summary.blocker_code.is_some()
                || summary
                    .downstream_dispatch_blockers
                    .iter()
                    .any(|value| !value.trim().is_empty());
            let status = if needs_status_projection {
                store.run_graph_status(run_id).await.ok()
            } else {
                None
            };
            let retired_closed_task_status =
                retired_closed_task_status_for_show(&store, status.as_ref()).await;
            let closed_task_retired = retired_closed_task_status.is_some();
            let status = retired_closed_task_status.or(status);
            let recovery = status.as_ref().map(|status| {
                crate::state_store::RunGraphRecoverySummary::from_status(status.clone())
            });
            let owned_write_scope_hint =
                task_owned_write_scope_for_status(&store, status.as_ref()).await;
            let summary = if closed_task_retired {
                retired_closed_task_summary_for_show(summary)
            } else {
                summary
            };
            let exception_path_metadata_path = if closed_task_retired {
                None
            } else {
                match exception_takeover_metadata_path(store.root(), run_id) {
                    Ok(path) => path.exists().then(|| path.display().to_string()),
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                }
            };
            let exception_path_metadata = if closed_task_retired {
                None
            } else {
                match read_exception_takeover_metadata(store.root(), run_id) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                }
            };
            let truth = derive_lane_show_truth_with_exception_metadata(
                &summary,
                recovery.as_ref(),
                exception_path_metadata.as_ref(),
            );
            let envelope = build_lane_envelope_with_owned_scope(
                summary,
                status,
                exception_path_metadata_path,
                exception_path_metadata,
                operator_session_projection,
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
                &owned_write_scope_hint,
            );
            return emit_lane_envelope_with_projection_cache(
                &state_dir, run_id, &envelope, *as_json,
            );
        }
        LaneCommand::TakeoverReady { run_id, as_json } => {
            let store = match StateStore::open_existing_read_only_with_timeout(
                state_dir.clone(),
                LANE_SURFACE_LOCK_TIMEOUT,
            )
            .await
            {
                Ok(store) => store,
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    return ExitCode::from(1);
                }
            };
            let operator_session_projection =
                match crate::operator_session_projection::build_operator_session_projection(&store)
                    .await
                {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("Failed to build operator session projection: {error}");
                        return ExitCode::from(1);
                    }
                };
            let Some(receipt) = (match store
                .run_graph_dispatch_receipt_for_status(run_id, None)
                .await
            {
                Ok(receipt) => receipt,
                Err(error) => {
                    eprintln!("Failed to read lane receipt `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                return emit_missing_lane_receipt_envelope(
                    *as_json,
                    Some(run_id),
                    "vida lane takeover-ready",
                );
            };
            let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let status = store.run_graph_status(run_id).await.ok();
            let recovery = status.as_ref().map(|status| {
                crate::state_store::RunGraphRecoverySummary::from_status(status.clone())
            });
            let owned_write_scope_hint =
                task_owned_write_scope_for_status(&store, status.as_ref()).await;
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
            let truth = derive_lane_show_truth_with_exception_metadata(
                &summary,
                recovery.as_ref(),
                exception_path_metadata.as_ref(),
            );
            let lane_envelope = build_lane_envelope_with_owned_scope(
                summary,
                status,
                exception_path_metadata_path
                    .exists()
                    .then(|| exception_path_metadata_path.display().to_string()),
                exception_path_metadata,
                operator_session_projection,
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
                &owned_write_scope_hint,
            );
            let takeover_envelope = build_lane_takeover_ready_envelope(lane_envelope);
            return emit_lane_takeover_ready_envelope(&takeover_envelope, *as_json);
        }
        _ => {}
    }

    let store = match StateStore::open_existing(state_dir.clone()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let operator_session_projection =
        match crate::operator_session_projection::build_operator_session_projection(&store).await {
            Ok(value) => value,
            Err(error) => {
                eprintln!("Failed to build operator session projection: {error}");
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
                return emit_missing_lane_receipt_envelope(as_json, None, "vida lane show");
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
            let truth = derive_lane_show_truth_with_exception_metadata(
                &summary,
                recovery.as_ref(),
                exception_path_metadata.as_ref(),
            );
            let envelope = build_lane_envelope(
                summary,
                status,
                exception_path_metadata_path
                    .exists()
                    .then(|| exception_path_metadata_path.display().to_string()),
                exception_path_metadata,
                operator_session_projection,
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
            );
            emit_lane_envelope_with_projection_cache(&state_dir, "latest", &envelope, as_json)
        }
        LaneCommand::ShowRun { run_id, as_json } => {
            let status = store.run_graph_status(run_id).await.ok();
            let Some(receipt) = (match store
                .run_graph_dispatch_receipt_for_status(run_id, status.as_ref())
                .await
            {
                Ok(receipt) => receipt,
                Err(error) => {
                    eprintln!("Failed to read lane receipt `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                return emit_missing_lane_receipt_envelope(as_json, Some(run_id), "vida lane show");
            };
            let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let recovery = status.as_ref().map(|status| {
                crate::state_store::RunGraphRecoverySummary::from_status(status.clone())
            });
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
            let truth = derive_lane_show_truth_with_exception_metadata(
                &summary,
                recovery.as_ref(),
                exception_path_metadata.as_ref(),
            );
            let envelope = build_lane_envelope(
                summary,
                status,
                exception_path_metadata_path
                    .exists()
                    .then(|| exception_path_metadata_path.display().to_string()),
                exception_path_metadata,
                operator_session_projection,
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
            );
            emit_lane_envelope_with_projection_cache(&state_dir, run_id, &envelope, as_json)
        }
        LaneCommand::TakeoverReady { .. } => {
            eprintln!("takeover-ready is a read-only lane command and should be handled before the writable lane store is opened.");
            ExitCode::from(2)
        }
        LaneCommand::Complete {
            run_id,
            receipt_id,
            host_bridge_request,
            host_agent_id,
            host_bridge_summary,
            state_dir: complete_state_dir,
            as_json,
        } => {
            let Some(mut receipt) = (match store.run_graph_dispatch_receipt(run_id).await {
                Ok(receipt) => receipt,
                Err(error) => {
                    eprintln!("Failed to read lane receipt `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                return emit_missing_lane_receipt_envelope(
                    as_json,
                    Some(run_id),
                    "vida lane complete",
                );
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
            let host_bridge_completion_context =
                if let Some(request_path) = host_bridge_request.as_deref() {
                    match trusted_host_bridge_completion_request_context(
                        store.root(),
                        run_id,
                        request_path,
                        status.as_ref(),
                        &receipt,
                    ) {
                        Ok(context) => context,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(2);
                        }
                    }
                } else {
                    None
                };
            let (packet_path, allow_dispatch_packet) = if let Some(context) =
                host_bridge_completion_context.as_ref()
            {
                receipt.dispatch_target = context.dispatch_target.clone();
                (context.packet_path.clone(), true)
            } else {
                let Some((packet_path, allow_dispatch_packet)) =
                    lane_completion_packet_path(&receipt)
                else {
                    eprintln!(
                            "Lane `{run_id}` has no persisted dispatch packet evidence for bounded completion."
                        );
                    return ExitCode::from(2);
                };
                (packet_path, allow_dispatch_packet)
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
            let mut packet = match read_lane_packet(store.root(), &validated_packet_path) {
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
            if takeover_active {
                if let Some(metadata) = exception_path_metadata.as_ref() {
                    let packet_template_kind = packet
                        .get("packet_template_kind")
                        .and_then(serde_json::Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string);
                    if let Some(packet_template_kind) = packet_template_kind {
                        if let Some(active_packet) = packet.get_mut(&packet_template_kind) {
                            if crate::runtime_dispatch_state::apply_owned_paths_if_missing(
                                active_packet,
                                &metadata.owned_write_scope,
                            ) {
                                if let Some(object) = packet.as_object_mut() {
                                    object.insert(
                                        "owned_paths".to_string(),
                                        serde_json::json!(metadata.owned_write_scope),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            if let Err(error) =
                crate::validate_runtime_dispatch_packet_contract(&packet, "Lane completion packet")
            {
                eprintln!("{error}");
                return ExitCode::from(2);
            }
            let host_bridge_evidence = if let Some(request_path) = host_bridge_request {
                let retrying_summary_guard = receipt.dispatch_status == "blocked"
                    && (receipt
                        .blocker_code
                        .as_deref()
                        .is_some_and(host_bridge_completion_retryable_blocker)
                        || receipt
                            .downstream_dispatch_blockers
                            .iter()
                            .any(|blocker| host_bridge_completion_retryable_blocker(blocker)));
                let retrying_request_guard = host_bridge_request_has_retryable_completion_evidence(
                    store.root(),
                    request_path,
                );
                if receipt.dispatch_status != "bridge_request_pending"
                    && !retrying_summary_guard
                    && !retrying_request_guard
                {
                    eprintln!("Lane `{run_id}` is not waiting on host bridge completion evidence.");
                    return ExitCode::from(2);
                }
                let taskflow_artifacts = taskflow_implementation_artifacts_for_host_bridge_request(
                    &store,
                    request_path,
                    run_id,
                )
                .await;
                let authoritative_owned_paths = owned_paths_from_lane_packet(&packet);
                match materialize_host_bridge_completion_evidence(
                    store.root(),
                    request_path,
                    run_id,
                    &receipt.dispatch_target,
                    &receipt,
                    receipt_id,
                    host_agent_id,
                    host_bridge_summary,
                    taskflow_artifacts,
                    &authoritative_owned_paths,
                    retrying_summary_guard || retrying_request_guard,
                    host_bridge_completion_context.is_some(),
                ) {
                    Ok(evidence) => Some(evidence),
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                }
            } else {
                if host_bridge_completion_request_required(&receipt) {
                    eprintln!(
                        "Lane `{run_id}` is waiting on host bridge completion evidence; pass --host-bridge-request with the pending request path."
                    );
                    return ExitCode::from(2);
                }
                None
            };
            let completed_target = packet
                .get("downstream_dispatch_active_target")
                .and_then(serde_json::Value::as_str)
                .or(receipt.downstream_dispatch_active_target.as_deref())
                .or(receipt.downstream_dispatch_last_target.as_deref())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(receipt.dispatch_target.as_str())
                .to_string();
            let completion_previous_target = packet
                .get("source_dispatch_target")
                .and_then(serde_json::Value::as_str)
                .or(receipt.downstream_dispatch_last_target.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let completion_blocker_code =
                crate::runtime_dispatch_state::runtime_lane_completion_summary_blocker_code(
                    &completed_target,
                    host_bridge_summary,
                );
            let completion_result_path =
                match crate::runtime_dispatch_state::write_runtime_lane_completion_result_with_summary(
                    store.root(),
                    run_id,
                    &completed_target,
                    receipt_id,
                    &validated_packet_path,
                    host_bridge_summary,
                ) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            let missing_owned_scope_handoff = receipt
                .downstream_dispatch_blockers
                .iter()
                .any(|blocker| blocker == "missing_owned_write_scope");
            let completion_blocked = completion_blocker_code.is_some()
                || host_bridge_evidence
                    .as_ref()
                    .is_some_and(|evidence| evidence.execution_state == "blocked");
            if completion_blocked {
                let mut blocker_codes = Vec::new();
                if let Some(evidence) = host_bridge_evidence.as_ref() {
                    blocker_codes.extend(evidence.blocker_codes.clone());
                }
                if let Some(completion_blocker_code) = completion_blocker_code.clone() {
                    blocker_codes.push(completion_blocker_code);
                }
                if blocker_codes.is_empty() {
                    blocker_codes.push("lane_completion_blocked_by_summary".to_string());
                }
                blocker_codes.sort();
                blocker_codes.dedup();
                let blocker_code = blocker_codes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "lane_completion_blocked_by_summary".to_string());
                receipt.downstream_dispatch_ready = false;
                receipt.downstream_dispatch_blockers = blocker_codes;
                receipt.downstream_dispatch_status = Some("blocked".to_string());
                receipt.dispatch_status = "blocked".to_string();
                receipt.blocker_code = Some(blocker_code);
                receipt.lane_status = crate::LaneStatus::LaneBlocked.as_str().to_string();
            } else {
                receipt.downstream_dispatch_ready = true;
                receipt.downstream_dispatch_blockers.clear();
                receipt.downstream_dispatch_status = Some("packet_ready".to_string());
                receipt.dispatch_status = "executed".to_string();
                receipt.blocker_code = None;
                receipt.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
            }
            receipt.downstream_dispatch_result_path = Some(completion_result_path.clone());
            receipt.downstream_dispatch_active_target = Some(completed_target.clone());
            receipt.downstream_dispatch_last_target =
                completion_previous_target.or_else(|| Some(completed_target.clone()));
            receipt.exception_path_receipt_id = None;
            receipt.supersedes_receipt_id = None;
            receipt.dispatch_result_path = Some(
                host_bridge_evidence
                    .as_ref()
                    .map(|evidence| evidence.result_path.clone())
                    .unwrap_or_else(|| completion_result_path.clone()),
            );
            if let Some(evidence) = host_bridge_evidence.as_ref() {
                receipt.dispatch_surface =
                    Some("vida lane complete --host-bridge-request".to_string());
                let mut dispatch_command = format!(
                    "vida lane complete {} --receipt-id {}",
                    crate::shell_quote(run_id),
                    crate::shell_quote(receipt_id)
                );
                if let Some(value) = host_bridge_request
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    dispatch_command.push_str(&format!(
                        " --host-bridge-request {}",
                        crate::shell_quote(value)
                    ));
                }
                if let Some(value) = host_agent_id
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    dispatch_command
                        .push_str(&format!(" --host-agent-id {}", crate::shell_quote(value)));
                }
                if let Some(value) = host_bridge_summary
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    dispatch_command.push_str(&format!(
                        " --host-bridge-summary {}",
                        crate::shell_quote(value)
                    ));
                }
                if let Some(value) = complete_state_dir
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    dispatch_command
                        .push_str(&format!(" --state-dir {}", crate::shell_quote(value)));
                }
                if as_json {
                    dispatch_command.push_str(" --json");
                }
                receipt.dispatch_command = Some(dispatch_command);
                receipt.downstream_dispatch_trace_path = Some(evidence.receipt_path.clone());
            }
            if !completion_blocked {
                match decode_lane_completion_packet_context(&packet) {
                    Ok(Some((role_selection, run_graph_bootstrap))) => {
                        let owned_paths_override = exception_path_metadata
                            .as_ref()
                            .filter(|_| takeover_active || missing_owned_scope_handoff)
                            .map(|metadata| metadata.owned_write_scope.as_slice())
                            .unwrap_or(&[]);
                        if let Err(error) = crate::runtime_dispatch_state::refresh_downstream_dispatch_preview_with_owned_paths(
                        &store,
                        &role_selection,
                        &run_graph_bootstrap,
                        &mut receipt,
                        owned_paths_override,
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
            if let Err(error) = store.record_run_graph_dispatch_receipt(&receipt).await {
                eprintln!("Failed to persist lane completion evidence: {error}");
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
            status = store.run_graph_status(run_id).await.ok();
            recovery = store.run_graph_recovery_summary(run_id).await.ok();

            let updated_summary =
                crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let truth = derive_lane_show_truth_with_exception_metadata(
                &updated_summary,
                recovery.as_ref(),
                exception_path_metadata.as_ref(),
            );
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
                operator_session_projection.clone(),
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
            );
            emit_lane_envelope_with_projection_cache(&state_dir, run_id, &envelope, as_json)
        }
        LaneCommand::Retire {
            run_id,
            receipt_id,
            reason: _reason,
            as_json,
        } => {
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
            let mut receipt = match store.run_graph_dispatch_receipt(run_id).await {
                Ok(Some(receipt)) => receipt,
                Ok(None) => {
                    let verdict = match crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(
                        &store,
                        &status,
                    )
                    .await
                    {
                        Ok(verdict) => verdict,
                        Err(error) => {
                            eprintln!(
                                "Failed to verify TaskFlow authority before retiring lane `{run_id}` without receipt: {error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                    if !verdict.task_missing() {
                        return emit_missing_lane_receipt_envelope(
                            as_json,
                            Some(run_id),
                            "vida lane retire",
                        );
                    }
                    match synthetic_missing_task_stale_run_receipt(store.root(), run_id, &status) {
                        Ok(receipt) => receipt,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to read lane receipt `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            };
            if receipt.lane_status == crate::LaneStatus::LaneExceptionRecorded.as_str()
                && !receipt
                    .supersedes_receipt_id
                    .as_deref()
                    .is_some_and(|receipt_id| !receipt_id.trim().is_empty())
            {
                eprintln!(
                    "Lane `{run_id}` has recorded exception evidence but no active exception takeover supersession; refusing retire."
                );
                return ExitCode::from(2);
            }
            let recovery = store.run_graph_recovery_summary(run_id).await.ok();
            if let Err(error) =
                lane_mutation_status_guard(run_id, Some(&status), recovery.as_ref(), &receipt)
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
                    let missing_task_stale_blocked_run = matches!(
                        error,
                        crate::state_store::StateStoreError::MissingTask { .. }
                    )
                        && missing_task_stale_blocked_run_can_retire(&status, &receipt);
                    if !missing_task_stale_blocked_run {
                        let metadata_task_id = if receipt.lane_status
                            == crate::LaneStatus::LaneExceptionTakeover.as_str()
                        {
                            exception_path_metadata
                                .as_ref()
                                .map(|metadata| metadata.active_bounded_unit.trim())
                                .filter(|task_id| !task_id.is_empty())
                        } else {
                            None
                        };
                        match metadata_task_id {
                            Some(task_id) => match store.show_task(task_id).await {
                                Ok(task) if task.status == "closed" => {}
                                Ok(task) => {
                                    eprintln!(
                                        "Lane `{run_id}` can only be retired after exception bounded unit `{}` is closed; current task status is `{}`.",
                                        task.id, task.status
                                    );
                                    return ExitCode::from(2);
                                }
                                Err(metadata_error) => {
                                    eprintln!(
                                        "Failed to verify exception bounded unit `{task_id}` before retiring lane `{run_id}` after run task `{}` lookup failed: {metadata_error}",
                                        status.task_id
                                    );
                                    return ExitCode::from(2);
                                }
                            },
                            None => {
                                eprintln!(
                                    "Failed to verify closed task `{}` before retiring lane `{run_id}`: {error}",
                                    status.task_id
                                );
                                return ExitCode::from(1);
                            }
                        }
                    }
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
            let truth = derive_lane_show_truth_with_exception_metadata(
                &updated_summary,
                recovery.as_ref(),
                exception_path_metadata.as_ref(),
            );
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
                Some(retired_status),
                exception_path_metadata_path
                    .exists()
                    .then(|| exception_path_metadata_path.display().to_string()),
                exception_path_metadata,
                operator_session_projection.clone(),
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
            );
            emit_lane_envelope_with_projection_cache(&state_dir, run_id, &envelope, as_json)
        }
        LaneCommand::ExceptionTakeover {
            run_id,
            receipt_id,
            metadata,
            activate,
            as_json,
        } => {
            let Some(mut receipt) = (match store.run_graph_dispatch_receipt(run_id).await {
                Ok(receipt) => receipt,
                Err(error) => {
                    eprintln!("Failed to read lane receipt `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                return emit_missing_lane_receipt_envelope(
                    as_json,
                    Some(run_id),
                    "vida lane exception-takeover",
                );
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
            let mut metadata = metadata;
            metadata.bind_to_receipt(&receipt);
            let metadata_path =
                match write_exception_takeover_metadata(store.root(), run_id, &metadata) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            if activate {
                receipt.supersedes_receipt_id = Some(receipt_id.to_string());
            }
            receipt.lane_status = explicit_lane_status_for_receipt(&receipt, recovery.as_ref());
            if let Err(error) = store.record_run_graph_dispatch_receipt(&receipt).await {
                eprintln!("Failed to persist exception takeover receipt: {error}");
                return ExitCode::from(1);
            }
            let updated_summary =
                crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let truth = derive_lane_show_truth_with_exception_metadata(
                &updated_summary,
                recovery.as_ref(),
                Some(&metadata),
            );
            let envelope = build_lane_envelope(
                updated_summary,
                status,
                Some(metadata_path),
                Some(metadata),
                operator_session_projection.clone(),
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
            );
            emit_lane_envelope_with_projection_cache(&state_dir, run_id, &envelope, as_json)
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
                return emit_missing_lane_receipt_envelope(
                    as_json,
                    Some(run_id),
                    "vida lane supersede",
                );
            };
            let recovery = store.run_graph_recovery_summary(run_id).await.ok();
            let status = store.run_graph_status(run_id).await.ok();
            if let Err(error) =
                lane_mutation_status_guard(run_id, status.as_ref(), recovery.as_ref(), &receipt)
            {
                eprintln!("{error}");
                return ExitCode::from(2);
            }
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
            if let Some(metadata) = exception_path_metadata.as_ref() {
                if let Err(error) = metadata.validate_for_receipt(&receipt) {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            } else if receipt
                .exception_path_receipt_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                eprintln!(
                    "Missing receipt-bound exception takeover metadata for lane `{run_id}`; record exception takeover before superseding."
                );
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
            let truth = derive_lane_show_truth_with_exception_metadata(
                &updated_summary,
                recovery.as_ref(),
                exception_path_metadata.as_ref(),
            );
            let envelope = build_lane_envelope(
                updated_summary,
                status,
                exception_path_metadata_path
                    .exists()
                    .then(|| exception_path_metadata_path.display().to_string()),
                exception_path_metadata,
                operator_session_projection.clone(),
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
            );
            emit_lane_envelope_with_projection_cache(&state_dir, run_id, &envelope, as_json)
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

    trait StateStoreFixtureTaskExt {
        fn create_task_with_fixture_parent<'a>(
            &'a self,
            request: crate::state_store::CreateTaskRequest<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::state_store::TaskRecord,
                            crate::state_store::StateStoreError,
                        >,
                    > + 'a,
            >,
        >;
    }

    impl StateStoreFixtureTaskExt for crate::StateStore {
        fn create_task_with_fixture_parent<'a>(
            &'a self,
            request: crate::state_store::CreateTaskRequest<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::state_store::TaskRecord,
                            crate::state_store::StateStoreError,
                        >,
                    > + 'a,
            >,
        > {
            Box::pin(async move {
                let crate::state_store::CreateTaskRequest {
                    task_id,
                    title,
                    display_id,
                    description,
                    issue_type,
                    status,
                    priority,
                    parent_id,
                    labels,
                    execution_semantics,
                    planner_metadata,
                    created_by,
                    source_repo,
                } = request;
                let generated_parent_id = (issue_type != "epic" && parent_id.is_none())
                    .then(|| format!("{task_id}-fixture-parent"));
                if let Some(parent_task_id) = generated_parent_id.as_deref() {
                    let parent_labels: Vec<String> = Vec::new();
                    let parent_status = if matches!(status.trim(), "closed" | "completed") {
                        "closed"
                    } else {
                        "open"
                    };
                    self.create_task(crate::state_store::CreateTaskRequest {
                        task_id: parent_task_id,
                        title: "Fixture parent epic",
                        display_id: None,
                        description: "Test-only parent epic for strict task hierarchy fixtures",
                        issue_type: "epic",
                        status: parent_status,
                        priority,
                        parent_id: None,
                        labels: &parent_labels,
                        execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                        planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                        created_by,
                        source_repo,
                    })
                    .await?;
                }
                self.create_task(crate::state_store::CreateTaskRequest {
                    task_id,
                    title,
                    display_id,
                    description,
                    issue_type,
                    status,
                    priority,
                    parent_id: parent_id.or(generated_parent_id.as_deref()),
                    labels,
                    execution_semantics,
                    planner_metadata,
                    created_by,
                    source_repo,
                })
                .await
            })
        }
    }

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
                host_bridge_request: None,
                host_agent_id: None,
                host_bridge_summary: None,
                state_dir: None,
                as_json: true
            }
        ));
    }

    #[test]
    fn parse_lane_complete_supports_host_bridge_evidence_options() {
        let args = vec![
            "complete".to_string(),
            "run-1".to_string(),
            "--receipt-id".to_string(),
            "receipt-1".to_string(),
            "--host-bridge-request".to_string(),
            ".vida/data/state/host-tool-bridge/requests/request-1.json".to_string(),
            "--host-agent-id".to_string(),
            "agent-1".to_string(),
            "--host-bridge-summary".to_string(),
            "agent completed".to_string(),
            "--state-dir".to_string(),
            ".vida/data/state".to_string(),
            "--json".to_string(),
        ];
        let command = parse_lane_args(&args).expect("host bridge lane complete should parse");
        assert!(matches!(
            command,
            LaneCommand::Complete {
                run_id: "run-1",
                receipt_id: "receipt-1",
                host_bridge_request: Some(
                    ".vida/data/state/host-tool-bridge/requests/request-1.json"
                ),
                host_agent_id: Some("agent-1"),
                host_bridge_summary: Some("agent completed"),
                state_dir: Some(".vida/data/state"),
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
                activate: false,
                as_json: true,
                ..
            }
        ));
    }

    #[test]
    fn parse_lane_exception_takeover_supports_atomic_activate() {
        let mut args = sample_exception_takeover_args("run-1", "receipt-1");
        args.insert(args.len() - 1, "--activate".to_string());
        let command = parse_lane_args(&args).expect("lane exception takeover should parse");
        assert!(matches!(
            command,
            LaneCommand::ExceptionTakeover {
                run_id: "run-1",
                receipt_id: "receipt-1",
                activate: true,
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
            .any(|action| action.contains("vida taskflow recovery status run-lane-test")));
        assert!(truth
            .next_actions
            .iter()
            .any(|action| action.contains("vida lane show run-lane-test")));
        assert!(truth
            .next_actions
            .iter()
            .all(|action| !action.contains("--json")));
    }

    #[test]
    fn lane_show_truth_keeps_materialization_only_receipt_blocked_when_recovery_is_clean() {
        let mut receipt = sample_receipt("blocked");
        receipt.dispatch_target = "work-pool-pack".to_string();
        receipt.lane_status = crate::LaneStatus::LaneBlocked.as_str().to_string();
        receipt.dispatch_surface = Some("vida task ensure".to_string());
        receipt.blocker_code = Some("internal_activation_view_only".to_string());
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-lane-test".to_string(),
            task_id: "task-lane-test".to_string(),
            active_node: "work-pool-pack".to_string(),
            lifecycle_stage: "work_pool_pack_complete".to_string(),
            resume_node: None,
            resume_status: "none".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "work-pool-pack".to_string(),
                delegated_cycle_open: false,
                delegated_cycle_state: "none".to_string(),
                local_exception_takeover_gate: "delegated_cycle_clear".to_string(),
                reporting_pause_gate: "not_required".to_string(),
                continuation_signal: "none".to_string(),
                blocker_code: None,
                lifecycle_stage: "work_pool_pack_complete".to_string(),
            },
        };

        let truth = derive_lane_show_truth(&summary, Some(&recovery));

        assert!(truth.blocked);
        assert!(truth
            .blocker_codes
            .contains(&"internal_activation_view_only".to_string()));
        assert!(truth
            .next_actions
            .iter()
            .any(|action| action.contains("vida taskflow recovery status run-lane-test")));
        assert!(truth
            .next_actions
            .iter()
            .all(|action| !action.contains("--json")));
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
            .contains(&"missing_owned_write_scope".to_string()));
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
            .contains(&"missing_owned_write_scope".to_string()));
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
    fn lane_envelope_surfaces_ready_downstream_agent_init_execute_command() {
        let mut receipt = sample_receipt("executed");
        receipt.blocker_code = None;
        receipt.lane_status = crate::LaneStatus::LaneRunning.as_str().to_string();
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_status = Some("packet_ready".to_string());
        receipt.downstream_dispatch_target = Some("writer".to_string());
        receipt.downstream_dispatch_command =
            Some("vida agent-init --downstream-packet packet.json --json".to_string());
        receipt.downstream_dispatch_packet_path = Some("packet.json".to_string());
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let truth = derive_lane_show_truth(&summary, None);

        let envelope = build_lane_envelope(
            summary,
            None,
            None,
            None,
            serde_json::json!({}),
            truth.blocked,
            truth.blocker_codes,
            truth.next_actions,
        );

        assert_eq!(envelope.status, "pass");
        assert!(envelope.next_actions.is_empty());
        assert_eq!(
            envelope.recommended_command.as_deref(),
            Some("vida agent-init --downstream-packet packet.json --execute-dispatch --json")
        );
        assert_eq!(
            envelope.recommended_surface.as_deref(),
            Some("vida agent-init")
        );
        assert_eq!(
            envelope
                .next_action
                .as_ref()
                .map(|action| action.command.as_str()),
            Some("vida agent-init --downstream-packet packet.json --execute-dispatch --json")
        );
    }

    #[test]
    fn lane_blocked_open_cycle_envelope_uses_task_owned_scope_in_takeover_command() {
        let mut receipt = sample_receipt("blocked");
        receipt.run_id = "run-lane-test".to_string();
        receipt.blocker_code = Some("internal_codex_windows_sandbox_unavailable".to_string());
        receipt.lane_status = crate::LaneStatus::LaneBlocked.as_str().to_string();
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-lane-test",
            "implementation",
            "writer",
        );
        status.active_node = "writer".to_string();
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-lane-test".to_string(),
            task_id: "task-lane-test".to_string(),
            active_node: "writer".to_string(),
            lifecycle_stage: "writer_blocked".to_string(),
            resume_node: None,
            resume_status: "running".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "handoff_pending".to_string(),
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "writer".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "delegated_cycle_open".to_string(),
                continuation_signal: "continue_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "writer_blocked".to_string(),
            },
        };
        let truth = derive_lane_show_truth(&summary, Some(&recovery));

        let owned_scope = vec!["crates/vida/src/lane_surface.rs".to_string()];
        let envelope = build_lane_envelope_with_owned_scope(
            summary,
            Some(status),
            None,
            None,
            serde_json::json!({}),
            truth.blocked,
            truth.blocker_codes,
            truth.next_actions,
            &owned_scope,
        );

        let command = envelope
            .recommended_command
            .as_deref()
            .expect("blocked lane should recommend a recovery command");
        assert!(command.starts_with(
            "vida lane exception-takeover run-lane-test --receipt-id run-lane-test-exception-takeover"
        ));
        assert!(command.contains("--reason-class internal_codex_windows_sandbox_unavailable"));
        assert!(command.contains("--active-bounded-unit run-lane-test:writer:exception-takeover"));
        assert!(command.contains("--owned-write-scope crates/vida/src/lane_surface.rs"));
        assert!(!command.contains("<owned-write-scope>"));
        assert!(command.contains("--verification-step"));
        assert_eq!(
            envelope.recommended_surface.as_deref(),
            Some("vida lane exception-takeover")
        );
        assert_eq!(
            envelope
                .next_action
                .as_ref()
                .map(|action| action.command.as_str()),
            envelope.recommended_command.as_deref()
        );
    }

    #[test]
    fn lane_blocked_open_cycle_without_scope_does_not_emit_placeholder_takeover_command() {
        let mut receipt = sample_receipt("blocked");
        receipt.run_id = "run-lane-test".to_string();
        receipt.blocker_code = Some("configured_backend_dispatch_failed".to_string());
        receipt.lane_status = crate::LaneStatus::LaneBlocked.as_str().to_string();
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-lane-test",
            "task-lane-test",
            "writer",
        );
        status.task_id = "task-lane-test".to_string();
        status.active_node = "writer".to_string();
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-lane-test".to_string(),
            task_id: "task-lane-test".to_string(),
            active_node: "writer".to_string(),
            lifecycle_stage: "writer_blocked".to_string(),
            resume_node: None,
            resume_status: "running".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "handoff_pending".to_string(),
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "writer".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "delegated_cycle_open".to_string(),
                continuation_signal: "continue_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "writer_blocked".to_string(),
            },
        };
        let truth = derive_lane_show_truth(&summary, Some(&recovery));

        let envelope = build_lane_envelope(
            summary,
            Some(status),
            None,
            None,
            serde_json::json!({}),
            truth.blocked,
            truth.blocker_codes,
            truth.next_actions,
        );

        let command = envelope
            .recommended_command
            .as_deref()
            .expect("blocked lane should provide concrete diagnostic guidance");
        assert_eq!(command, "vida task show task-lane-test --json");
        assert!(!command.contains("<owned-write-scope>"));
        assert_eq!(
            envelope.recommended_surface.as_deref(),
            Some("vida task show")
        );
    }

    #[test]
    fn lane_recorded_exception_envelope_exposes_supersede_command() {
        let mut receipt = sample_receipt("blocked");
        receipt.run_id = "run-recorded-exception".to_string();
        receipt.exception_path_receipt_id = Some("receipt-1".to_string());
        receipt.lane_status = crate::LaneStatus::LaneExceptionRecorded
            .as_str()
            .to_string();
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let truth = derive_lane_show_truth(&summary, None);

        let envelope = build_lane_envelope(
            summary,
            None,
            None,
            None,
            serde_json::json!({}),
            truth.blocked,
            truth.blocker_codes,
            truth.next_actions,
        );

        assert_eq!(
            envelope.recommended_command.as_deref(),
            Some("vida lane supersede run-recorded-exception --receipt-id receipt-1 --json")
        );
        assert_eq!(
            envelope.recommended_surface.as_deref(),
            Some("vida lane supersede")
        );
    }

    #[test]
    fn lane_show_rejects_state_stale_blocker_projection() {
        let root = std::env::temp_dir().join(format!(
            "vida-lane-stale-projection-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let run_id = "run-lane-stale";
        crate::operator_projection_cache::write_json_projection(
            &root,
            &lane_show_projection_name(run_id),
            &serde_json::json!({
                "surface": "vida lane",
                "status": "blocked",
                "blocker_codes": ["open_delegated_cycle"],
                "lane_status": "lane_blocked"
            }),
        );
        crate::operator_projection_cache::touch_state_mutation_marker(&root);

        assert!(
            read_cached_lane_show_projection(&root, &lane_show_projection_name(run_id)).is_none()
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn build_lane_envelope_exposes_root_scope_only_for_active_takeover() {
        let metadata = ExceptionTakeoverMetadata {
            run_id: Some("run-lane-test".to_string()),
            dispatch_target: Some("spec-pack".to_string()),
            dispatch_packet_path: None,
            source_exception_path_receipt_id: Some("exception-1".to_string()),
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
            serde_json::json!({}),
            false,
            Vec::new(),
            Vec::new(),
        );
        assert!(stale_envelope
            .root_local_write_allowed_for_only_these_paths
            .is_empty());
        assert!(!stale_envelope.root_local_write_allowed);
        assert!(stale_envelope.owned_write_scope.is_empty());
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
            serde_json::json!({}),
            false,
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(
            active_envelope.root_local_write_allowed_for_only_these_paths,
            vec!["crates/vida/src/lane_surface.rs".to_string()]
        );
        assert!(active_envelope.root_local_write_allowed);
        assert_eq!(
            active_envelope.owned_write_scope,
            vec!["crates/vida/src/lane_surface.rs".to_string()]
        );
        assert_eq!(
            active_envelope.artifact_refs["root_local_write_allowed"],
            true
        );
        assert_eq!(
            active_envelope.artifact_refs["owned_write_scope"],
            serde_json::json!(["crates/vida/src/lane_surface.rs"])
        );
    }

    #[test]
    fn closed_lane_exception_state_does_not_keep_stale_takeover_authority() {
        let metadata = ExceptionTakeoverMetadata {
            run_id: Some("run-lane-test".to_string()),
            dispatch_target: Some("spec-pack".to_string()),
            dispatch_packet_path: None,
            source_exception_path_receipt_id: Some("exception-1".to_string()),
            reason_class: "test".to_string(),
            active_bounded_unit: "closed-lane-exception-state-stale-defect".to_string(),
            owned_write_scope: vec!["crates/vida/src/lane_surface.rs".to_string()],
            why_delegated_or_rerouted_path_is_not_currently_lawful: "blocked".to_string(),
            why_local_write_is_the_smallest_safe_bounded_workaround: "bounded".to_string(),
            return_to_normal_posture_condition: "closed task is terminal".to_string(),
            verification_plan: vec!["cargo test -p vida closed_lane_exception".to_string()],
            recorded_at: "2026-06-03T00:00:00Z".to_string(),
        };
        let mut receipt = sample_receipt("executed");
        receipt.blocker_code = None;
        receipt.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.supersedes_receipt_id = Some("exception-1".to_string());
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-lane-test".to_string(),
            task_id: "run-lane-test".to_string(),
            active_node: "coach".to_string(),
            lifecycle_stage: "closure_complete".to_string(),
            resume_node: None,
            resume_status: "completed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "coach".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_blocked".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "delegated_cycle_open".to_string(),
                continuation_signal: "continue_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "coach_blocked".to_string(),
            },
        };

        let truth = derive_lane_show_truth_with_exception_metadata(
            &summary,
            Some(&recovery),
            Some(&metadata),
        );
        assert!(active_exception_write_scope(&summary, Some(&metadata)).is_empty());
        let envelope = build_lane_envelope(
            summary,
            None,
            Some("/tmp/exception.json".to_string()),
            Some(metadata),
            serde_json::json!({}),
            truth.blocked,
            truth.blocker_codes,
            truth.next_actions,
        );

        assert_eq!(envelope.status, "pass");
        assert!(!envelope.root_local_write_allowed);
        assert!(envelope.owned_write_scope.is_empty());
        assert!(envelope
            .root_local_write_allowed_for_only_these_paths
            .is_empty());
        assert!(!envelope
            .blocker_codes
            .contains(&"open_delegated_cycle".to_string()));
    }

    #[test]
    fn takeover_ready_envelope_reports_active_write_scope() {
        let metadata = ExceptionTakeoverMetadata {
            run_id: Some("run-lane-test".to_string()),
            dispatch_target: Some("implementer".to_string()),
            dispatch_packet_path: None,
            source_exception_path_receipt_id: Some("exception-1".to_string()),
            reason_class: "pending_implementation_evidence".to_string(),
            active_bounded_unit: "run-lane-test:implementer:exception-takeover".to_string(),
            owned_write_scope: vec!["crates/vida/src/lane_surface.rs".to_string()],
            why_delegated_or_rerouted_path_is_not_currently_lawful: "delegated lane is blocked"
                .to_string(),
            why_local_write_is_the_smallest_safe_bounded_workaround: "bounded owned scope only"
                .to_string(),
            return_to_normal_posture_condition: "focused proof passes".to_string(),
            verification_plan: vec!["cargo test -p vida lane_surface".to_string()],
            recorded_at: "2026-06-02T00:00:00Z".to_string(),
        };
        let mut receipt = sample_receipt("blocked");
        receipt.run_id = "run-lane-test".to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.lane_status = crate::LaneStatus::LaneExceptionTakeover
            .as_str()
            .to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.supersedes_receipt_id = Some("exception-1".to_string());
        receipt.downstream_dispatch_blockers = vec!["pending_implementation_evidence".to_string()];
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-lane-test",
            "implementation",
            "implementer",
        );
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        let mut recovery = crate::state_store::RunGraphRecoverySummary::from_status(status.clone());
        recovery.delegation_gate.local_exception_takeover_gate =
            "blocked_open_delegated_cycle".to_string();
        recovery.delegation_gate.delegated_cycle_open = true;
        let truth = derive_lane_show_truth(&summary, Some(&recovery));
        let lane_envelope = build_lane_envelope(
            summary,
            Some(status),
            Some("/tmp/exception.json".to_string()),
            Some(metadata),
            serde_json::json!({}),
            truth.blocked,
            truth.blocker_codes,
            truth.next_actions,
        );

        let takeover = build_lane_takeover_ready_envelope(lane_envelope);

        assert_eq!(takeover.status, "pass");
        assert_eq!(takeover.takeover_state, "active");
        assert!(takeover.takeover_ready);
        assert!(takeover.root_local_write_allowed);
        assert_eq!(
            takeover.owned_write_scope,
            vec!["crates/vida/src/lane_surface.rs".to_string()]
        );
        assert_eq!(
            takeover.artifact_refs["root_local_write_allowed"],
            serde_json::json!(true)
        );
        assert_eq!(
            takeover.artifact_refs["owned_write_scope"],
            serde_json::json!(["crates/vida/src/lane_surface.rs"])
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
            .contains(&"internal_dispatch_timeout_without_receipt".to_string()));
        assert!(truth
            .next_actions
            .iter()
            .any(|value| value.contains("finish the bounded exception unit")));
    }

    #[test]
    fn coach_exception_supersede_active_takeover_with_owned_scope_clears_open_cycle() {
        let mut receipt = sample_receipt("blocked");
        receipt.lane_status = crate::LaneStatus::LaneExceptionTakeover
            .as_str()
            .to_string();
        receipt.dispatch_target = "coach".to_string();
        receipt.exception_path_receipt_id = Some("coach-exception-1".to_string());
        receipt.supersedes_receipt_id = Some("coach-exception-1".to_string());
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
        status.lifecycle_stage = "coach_blocked".to_string();
        status.active_node = "coach".to_string();
        let mut recovery = crate::state_store::RunGraphRecoverySummary::from_status(status);
        recovery.delegation_gate.local_exception_takeover_gate =
            "blocked_open_delegated_cycle".to_string();
        recovery.delegation_gate.delegated_cycle_open = true;
        let metadata = ExceptionTakeoverMetadata {
            run_id: Some("run-lane-test".to_string()),
            dispatch_target: Some("coach".to_string()),
            dispatch_packet_path: None,
            source_exception_path_receipt_id: Some("coach-exception-1".to_string()),
            reason_class: "external_coach_timeout_internal_approval".to_string(),
            active_bounded_unit: "run-lane-test:coach:exception-takeover".to_string(),
            owned_write_scope: vec!["crates/vida/src/lane_surface.rs".to_string()],
            why_delegated_or_rerouted_path_is_not_currently_lawful: "external coach timed out"
                .to_string(),
            why_local_write_is_the_smallest_safe_bounded_workaround:
                "internal coach approval exists".to_string(),
            return_to_normal_posture_condition: "focused supersede proof passes".to_string(),
            verification_plan: vec!["cargo test -p vida coach_exception_supersede".to_string()],
            recorded_at: "2026-06-03T00:00:00Z".to_string(),
        };

        let truth = derive_lane_show_truth_with_exception_metadata(
            &summary,
            Some(&recovery),
            Some(&metadata),
        );

        assert!(!truth.blocked);
        assert!(truth.blocker_codes.is_empty());
        assert!(truth.next_actions.is_empty());
    }

    #[tokio::test]
    async fn coach_exception_supersede_json_cache_clears_followup_lane_show() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-coach-supersede-cache-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-coach-supersede-cache";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "implementation", "coach");
        status.task_id = run_id.to_string();
        status.active_node = "coach".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "coach_blocked".to_string();
        status.resume_target = "dispatch.coach".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "coach".to_string();
        receipt.lane_status = crate::LaneStatus::LaneExceptionRecorded
            .as_str()
            .to_string();
        receipt.exception_path_receipt_id = Some("coach-exception-1".to_string());
        receipt.blocker_code = Some("internal_dispatch_timeout_without_receipt".to_string());
        receipt
            .downstream_dispatch_blockers
            .push("internal_dispatch_timeout_without_receipt".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        let metadata = ExceptionTakeoverMetadata {
            run_id: Some(run_id.to_string()),
            dispatch_target: Some("coach".to_string()),
            dispatch_packet_path: receipt.dispatch_packet_path.clone(),
            source_exception_path_receipt_id: Some("coach-exception-1".to_string()),
            reason_class: "external_coach_timeout_internal_approval".to_string(),
            active_bounded_unit: format!("{run_id}:coach:exception-takeover"),
            owned_write_scope: vec!["crates/vida/src/lane_surface.rs".to_string()],
            why_delegated_or_rerouted_path_is_not_currently_lawful: "external coach timed out"
                .to_string(),
            why_local_write_is_the_smallest_safe_bounded_workaround:
                "internal coach approval exists".to_string(),
            return_to_normal_posture_condition: "focused supersede proof passes".to_string(),
            verification_plan: vec!["cargo test -p vida coach_exception_supersede".to_string()],
            recorded_at: "2026-06-03T00:00:00Z".to_string(),
        };
        write_exception_takeover_metadata(store.root(), run_id, &metadata)
            .expect("persist exception metadata");
        drop(store);
        wait_for_state_unlock(&root);

        let supersede_args = ProxyArgs {
            args: vec![
                "supersede".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "coach-exception-1".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(supersede_args).await, ExitCode::SUCCESS);

        let show_args = ProxyArgs {
            args: vec!["show".to_string(), run_id.to_string(), "--json".to_string()],
        };
        assert_eq!(run_lane(show_args).await, ExitCode::SUCCESS);

        let cached = read_cached_lane_show_projection(&root, &lane_show_projection_name(run_id))
            .expect("lane show projection cache should be written");
        let cached_json: serde_json::Value =
            serde_json::from_str(&cached).expect("cached lane show projection should be json");
        assert_eq!(cached_json["status"], "pass");
        assert_eq!(cached_json["blocker_codes"], serde_json::json!([]));
        assert_eq!(
            cached_json["supersedes_receipt_id"],
            serde_json::json!("coach-exception-1")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn derive_lane_show_truth_accepts_superseded_exception_with_stale_recorded_lane_status() {
        let mut receipt = sample_receipt("blocked");
        receipt.lane_status = crate::LaneStatus::LaneExceptionRecorded
            .as_str()
            .to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.supersedes_receipt_id = Some("exception-1".to_string());
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-lane-test",
            "implementation",
            "analysis",
        );
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();
        let mut recovery = crate::state_store::RunGraphRecoverySummary::from_status(status);
        recovery.delegation_gate.local_exception_takeover_gate =
            "delegated_cycle_clear".to_string();
        recovery.delegation_gate.delegated_cycle_open = false;

        let truth = derive_lane_show_truth(&summary, Some(&recovery));

        assert!(!truth.blocked);
        assert!(truth.blocker_codes.is_empty());
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
        assert!(truth.next_actions.iter().any(|value| {
            value.contains("vida lane supersede run-lane-test --receipt-id exception-1 --json")
        }));
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
        let mut metadata = ExceptionTakeoverMetadata {
            run_id: None,
            dispatch_target: None,
            dispatch_packet_path: None,
            source_exception_path_receipt_id: None,
            reason_class: "blocked_open_delegated_cycle_timeout".to_string(),
            active_bounded_unit: format!("{run_id}:implementer:exception-takeover"),
            owned_write_scope: vec!["crates/vida/src/lane_surface.rs".to_string()],
            why_delegated_or_rerouted_path_is_not_currently_lawful: "blocked".to_string(),
            why_local_write_is_the_smallest_safe_bounded_workaround: "bounded".to_string(),
            return_to_normal_posture_condition: "verified".to_string(),
            verification_plan: vec!["test".to_string()],
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };
        metadata.bind_to_receipt(&receipt);
        write_exception_takeover_metadata(store.root(), run_id, &metadata)
            .expect("metadata should persist");
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
    async fn lane_supersede_rejects_stale_exception_metadata_without_mutation() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-supersede-stale-metadata-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-supersede-stale";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "scope_discussion",
        );
        status.active_node = "coach".to_string();
        status.lifecycle_stage = "coach_blocked".to_string();
        status.status = "blocked".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "coach".to_string();
        receipt.exception_path_receipt_id = Some("exception-current".to_string());
        receipt.lane_status = crate::LaneStatus::LaneExceptionRecorded
            .as_str()
            .to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist exception-recorded receipt");

        let stale_metadata = ExceptionTakeoverMetadata {
            run_id: Some(run_id.to_string()),
            dispatch_target: Some("implementer".to_string()),
            dispatch_packet_path: None,
            source_exception_path_receipt_id: Some("exception-old".to_string()),
            reason_class: "blocked_open_delegated_cycle_timeout".to_string(),
            active_bounded_unit: format!(
                "{run_id}:graph-summary-pass-operator-contract-next-actions"
            ),
            owned_write_scope: vec!["crates/vida/src".to_string()],
            why_delegated_or_rerouted_path_is_not_currently_lawful: "old blocker".to_string(),
            why_local_write_is_the_smallest_safe_bounded_workaround: "old bounded unit".to_string(),
            return_to_normal_posture_condition: "old verification".to_string(),
            verification_plan: vec!["old test".to_string()],
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };
        write_exception_takeover_metadata(store.root(), run_id, &stale_metadata)
            .expect("stale metadata should persist");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "supersede".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "supersede-current".to_string(),
                "--json".to_string(),
            ],
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
        assert!(after.supersedes_receipt_id.is_none());
        assert_eq!(after.lane_status, "lane_exception_recorded");

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
    async fn lane_exception_takeover_activate_records_and_activates_local_write() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-activate-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-activate";

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

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "spec-pack".to_string();
        receipt.blocker_code = Some("missing_execution_preparation_contract".to_string());
        receipt.lane_status = crate::LaneStatus::LaneBlocked.as_str().to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let mut args = sample_exception_takeover_args(run_id, "receipt-activate-1");
        args.insert(args.len() - 1, "--activate".to_string());
        assert_eq!(run_lane(ProxyArgs { args }).await, ExitCode::SUCCESS);

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
            Some("receipt-activate-1")
        );
        assert_eq!(
            after.supersedes_receipt_id.as_deref(),
            Some("receipt-activate-1")
        );
        assert_eq!(after.lane_status, "lane_exception_takeover");

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
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
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
    async fn lane_retire_allows_missing_task_stale_blocked_run() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-retire-missing-task-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-retire-missing-task";
        let task_id = "task-lane-retire-missing";

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
        let packet_path = packet_dir.join("run-lane-retire-missing-task.json");
        std::fs::write(
            &packet_path,
            "{\"run_id\":\"run-lane-retire-missing-task\"}",
        )
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
                    recorded_at: "2026-05-18T00:00:00Z".to_string(),
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
                "retire-missing-task-1".to_string(),
                "--reason".to_string(),
                "missing task stale run".to_string(),
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
        assert!(store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read continuation binding")
            .is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_retire_synthesizes_receipt_for_missing_task_stale_run_without_receipt() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-retire-missing-task-no-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-retire-missing-task-no-receipt";
        let task_id = "task-lane-retire-missing-no-receipt";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            task_id,
            "implementation",
            "implementation",
        );
        status.run_id = run_id.to_string();
        status.active_node = "analysis".to_string();
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementation_dispatch_ready".to_string();
        status.policy_gate = "host_tool_bridge_adapter_required".to_string();
        status.handoff_state = "handoff_pending".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.implementer".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist ready missing-task run graph status");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "retire".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "retire-missing-task-no-receipt-1".to_string(),
                "--reason".to_string(),
                "missing task stale run".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::SUCCESS);

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after synthetic retire");
        let retired = store
            .run_graph_status(run_id)
            .await
            .expect("read retired status");
        assert_eq!(retired.status, "completed");
        assert_eq!(retired.lifecycle_stage, "closure_complete");
        let receipt = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read synthetic retired receipt")
            .expect("synthetic receipt should exist");
        assert_eq!(receipt.dispatch_kind, "stale_run_retire");
        assert_eq!(
            receipt.lane_status,
            crate::LaneStatus::LaneCompleted.as_str()
        );
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("retired_closed_task_run")
        );
        assert!(receipt.dispatch_packet_path.is_some());
        assert!(store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read continuation binding")
            .is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_retire_allows_missing_task_stale_prelaunch_packet_run() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-retire-missing-task-prelaunch-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-retire-missing-task-prelaunch";
        let task_id = "task-lane-retire-missing-prelaunch";

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
            .expect("persist blocked prelaunch run graph status");

        let packet_dir = root.join("runtime-consumption").join("dispatch-packets");
        std::fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("run-lane-retire-missing-task-prelaunch.json");
        std::fs::write(
            &packet_path,
            "{\"run_id\":\"run-lane-retire-missing-task-prelaunch\"}",
        )
        .expect("write dispatch packet");

        let mut receipt = sample_receipt("executed");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.dispatch_result_path =
            Some("runtime-consumption/dispatch-results/prelaunch.json".to_string());
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_status = Some("packet_ready".to_string());
        receipt.downstream_dispatch_active_target = Some("analysis".to_string());
        receipt.downstream_dispatch_last_target = Some("analysis".to_string());
        receipt.blocker_code = None;
        receipt.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist prelaunch receipt");
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
                    binding_source: "test-prelaunch".to_string(),
                    why_this_unit: "test prelaunch binding".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
                    request_text: None,
                    recorded_at: "2026-05-21T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist prelaunch continuation binding");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "retire".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "retire-missing-task-prelaunch-1".to_string(),
                "--reason".to_string(),
                "missing TaskFlow task stale run".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::SUCCESS);

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after prelaunch retire");
        let retired = store
            .run_graph_status(run_id)
            .await
            .expect("read retired prelaunch status");
        assert_eq!(retired.status, "completed");
        assert_eq!(retired.lifecycle_stage, "closure_complete");
        let receipt = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read retired prelaunch receipt")
            .expect("receipt should exist");
        assert_eq!(
            receipt.lane_status,
            crate::LaneStatus::LaneCompleted.as_str()
        );
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("retired_closed_task_run")
        );
        assert!(store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read prelaunch continuation binding")
            .is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_retire_uses_exception_metadata_closed_unit_when_run_task_is_runtime_id() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-retire-runtime-id-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "runtime-vida-taskflow-codex";
        let bounded_task_id = "taskflow-defect-operator-surfaces-over-2s-read-model-projection";

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: bounded_task_id,
                title: "Closed exception bounded unit",
                display_id: None,
                description: "",
                issue_type: "defect",
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
            .expect("create closed bounded task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "coach".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "coach_blocked".to_string();
        status.policy_gate = "review_findings".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked runtime-id status");

        let packet_dir = root.join("runtime-consumption").join("dispatch-packets");
        std::fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("runtime-vida-taskflow-codex.json");
        std::fs::write(&packet_path, "{\"run_id\":\"runtime-vida-taskflow-codex\"}")
            .expect("write dispatch packet");

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.lane_status = crate::LaneStatus::LaneExceptionTakeover
            .as_str()
            .to_string();
        receipt.exception_path_receipt_id = Some(run_id.to_string());
        receipt.supersedes_receipt_id = Some(run_id.to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist exception takeover receipt");
        let mut metadata = ExceptionTakeoverMetadata {
            run_id: None,
            dispatch_target: None,
            dispatch_packet_path: None,
            source_exception_path_receipt_id: None,
            reason_class: "blocked_open_delegated_cycle_timeout".to_string(),
            active_bounded_unit: bounded_task_id.to_string(),
            owned_write_scope: vec!["crates/vida/src".to_string()],
            why_delegated_or_rerouted_path_is_not_currently_lawful: "blocked".to_string(),
            why_local_write_is_the_smallest_safe_bounded_workaround: "bounded".to_string(),
            return_to_normal_posture_condition: "verified".to_string(),
            verification_plan: vec!["test".to_string()],
            recorded_at: "2026-05-18T00:00:00Z".to_string(),
        };
        metadata.bind_to_receipt(&receipt);
        write_exception_takeover_metadata(store.root(), run_id, &metadata)
            .expect("metadata should persist");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "retire".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "retire-runtime-id-1".to_string(),
                "--reason".to_string(),
                "closed exception unit".to_string(),
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
        let receipt = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read retired receipt")
            .expect("receipt should exist");
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("retired_closed_task_run")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_retire_rejects_exception_takeover_missing_task_stale_blocked_run_without_closed_unit(
    ) {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-retire-exception-missing-task-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "runtime-audit-remediation-correct-report-baseline-publis";
        let missing_task_id = "audit-remediation-04-correct-report-baseline";
        let missing_exception_unit = "audit-remediation-04-correct-report-baseline-review";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            missing_task_id,
            "implementation",
            "implementation",
        );
        status.run_id = run_id.to_string();
        status.active_node = "coach".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "coach_blocked".to_string();
        status.policy_gate = "host_tool_bridge_adapter_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "coach".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked missing-task status");

        let packet_dir = root.join("runtime-consumption").join("dispatch-packets");
        std::fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("runtime-audit-remediation-correct-report-baseline.json");
        std::fs::write(
            &packet_path,
            format!(
                "{{\"run_id\":\"{run_id}\",\"delivery_task_packet\":{{\"task_id\":\"{missing_task_id}\"}}}}"
            ),
        )
        .expect("write exception takeover dispatch packet");

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_status = "blocked".to_string();
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.lane_status = crate::LaneStatus::LaneExceptionTakeover
            .as_str()
            .to_string();
        receipt.exception_path_receipt_id = Some(run_id.to_string());
        receipt.supersedes_receipt_id = Some(run_id.to_string());
        receipt.blocker_code = Some("host_tool_bridge_adapter_required".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist exception takeover receipt");

        let mut metadata = ExceptionTakeoverMetadata {
            run_id: None,
            dispatch_target: None,
            dispatch_packet_path: None,
            source_exception_path_receipt_id: None,
            reason_class: "blocked_open_delegated_cycle_timeout".to_string(),
            active_bounded_unit: missing_exception_unit.to_string(),
            owned_write_scope: vec!["crates/vida/src".to_string()],
            why_delegated_or_rerouted_path_is_not_currently_lawful: "blocked".to_string(),
            why_local_write_is_the_smallest_safe_bounded_workaround: "bounded".to_string(),
            return_to_normal_posture_condition: "verified".to_string(),
            verification_plan: vec!["test".to_string()],
            recorded_at: "2026-06-04T00:00:00Z".to_string(),
        };
        metadata.bind_to_receipt(&receipt);
        write_exception_takeover_metadata(store.root(), run_id, &metadata)
            .expect("exception metadata should persist");

        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: missing_task_id.to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "run_graph_task",
                        "task_id": missing_task_id,
                        "run_id": run_id,
                        "active_node": "coach"
                    }),
                    binding_source: "test-exception-missing-task".to_string(),
                    why_this_unit: "test exception missing-task stale run".to_string(),
                    primary_path: "exception_takeover_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
                    request_text: None,
                    recorded_at: "2026-06-04T00:00:01Z".to_string(),
                },
            )
            .await
            .expect("persist continuation binding");
        let seeded_status = store
            .run_graph_status(run_id)
            .await
            .expect("read seeded status");
        let seeded_receipt = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read seeded receipt")
            .expect("receipt should exist");
        let seeded_binding = store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read seeded continuation binding")
            .expect("binding should remain present");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "retire".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                run_id.to_string(),
                "--reason".to_string(),
                "missing TaskFlow task stale run".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::from(2));

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after exception retire rejection");
        let retained = store
            .run_graph_status(run_id)
            .await
            .expect("read retained status");
        assert_eq!(retained.status, seeded_status.status);
        assert_eq!(retained.lifecycle_stage, seeded_status.lifecycle_stage);
        assert_eq!(retained.resume_target, seeded_status.resume_target);
        assert_eq!(retained.recovery_ready, seeded_status.recovery_ready);
        let receipt = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read retained receipt")
            .expect("receipt should exist");
        assert_eq!(receipt.dispatch_status, seeded_receipt.dispatch_status);
        assert_eq!(receipt.lane_status, seeded_receipt.lane_status);
        assert_eq!(receipt.dispatch_target, seeded_receipt.dispatch_target);
        assert_eq!(
            receipt.dispatch_packet_path,
            seeded_receipt.dispatch_packet_path
        );
        let binding = store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read continuation binding")
            .expect("binding should remain present");
        assert_eq!(binding.status, seeded_binding.status);
        assert_eq!(binding.task_id, seeded_binding.task_id);
        assert_eq!(
            binding.active_bounded_unit,
            seeded_binding.active_bounded_unit
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_retire_rejects_bridge_pending_missing_task_stale_blocked_run() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-retire-bridge-pending-missing-task-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "runtime-resume-lane-governance-conflict-state-dir";

        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "analysis", "analysis");
        status.run_id = run_id.to_string();
        status.active_node = "analysis".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();
        status.policy_gate = "host_tool_bridge_adapter_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "analysis".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist bridge-pending missing-task status");

        let packet_dir = root.join("runtime-consumption").join("dispatch-packets");
        std::fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("runtime-resume-lane-governance-conflict-state-dir.json");
        std::fs::write(&packet_path, format!("{{\"run_id\":\"{run_id}\"}}"))
            .expect("write bridge-pending dispatch packet");

        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_status = "bridge_request_pending".to_string();
        receipt.lane_status = crate::LaneStatus::LaneOpen.as_str().to_string();
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.blocker_code = Some("host_tool_bridge_adapter_required".to_string());
        receipt.dispatch_target = "analysis".to_string();
        receipt.activation_runtime_role = Some("analysis".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist bridge-pending receipt");

        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: run_id.to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "run_graph_task",
                        "task_id": run_id,
                        "run_id": run_id,
                        "active_node": "analysis"
                    }),
                    binding_source: "test-bridge-pending-missing-task".to_string(),
                    why_this_unit: "test bridge-pending missing-task stale run".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
                    request_text: None,
                    recorded_at: "2026-06-05T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist continuation binding");
        let seeded_status = store
            .run_graph_status(run_id)
            .await
            .expect("read seeded status");
        let seeded_receipt = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read seeded receipt")
            .expect("receipt should exist");
        let seeded_binding = store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read seeded continuation binding")
            .expect("binding should remain present");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "retire".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                run_id.to_string(),
                "--reason".to_string(),
                "missing TaskFlow task stale run".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::from(1));

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after bridge-pending retire rejection");
        let retained = store
            .run_graph_status(run_id)
            .await
            .expect("read retained status");
        assert_eq!(retained.status, seeded_status.status);
        assert_eq!(retained.lifecycle_stage, seeded_status.lifecycle_stage);
        assert_eq!(retained.resume_target, seeded_status.resume_target);
        assert_eq!(retained.recovery_ready, seeded_status.recovery_ready);
        let receipt = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read retained receipt")
            .expect("receipt should exist");
        assert_eq!(receipt.dispatch_status, seeded_receipt.dispatch_status);
        assert_eq!(receipt.lane_status, seeded_receipt.lane_status);
        assert_eq!(receipt.dispatch_target, seeded_receipt.dispatch_target);
        assert_eq!(
            receipt.dispatch_packet_path,
            seeded_receipt.dispatch_packet_path
        );
        let binding = store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read continuation binding")
            .expect("binding should remain present");
        assert_eq!(binding.status, seeded_binding.status);
        assert_eq!(binding.task_id, seeded_binding.task_id);
        assert_eq!(
            binding.active_bounded_unit,
            seeded_binding.active_bounded_unit
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_retire_rejects_active_exception_takeover_missing_unit_stale_blocked_run() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-retire-active-exception-missing-unit-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "universal-surfaces-kanban-cross-column-drag-drop";
        let missing_exception_unit =
            "universal-surfaces-kanban-cross-column-drag-drop:implementer:exception-takeover";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.run_id = run_id.to_string();
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        status.policy_gate = "host_tool_bridge_adapter_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "implementer".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist active exception missing-unit status");

        let packet_dir = root.join("runtime-consumption").join("dispatch-packets");
        std::fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("universal-surfaces-kanban-cross-column-drag-drop.json");
        std::fs::write(
            &packet_path,
            format!(
                "{{\"run_id\":\"{run_id}\",\"delivery_task_packet\":{{\"task_id\":\"{run_id}\"}}}}"
            ),
        )
        .expect("write active exception dispatch packet");

        let mut receipt = sample_receipt("executed");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_status = "executed".to_string();
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.lane_status = crate::LaneStatus::LaneExceptionTakeover
            .as_str()
            .to_string();
        receipt.exception_path_receipt_id = Some(run_id.to_string());
        receipt.supersedes_receipt_id = Some(run_id.to_string());
        receipt.blocker_code = Some("host_tool_bridge_adapter_required".to_string());
        receipt.dispatch_target = "implementer".to_string();
        receipt.activation_runtime_role = Some("implementer".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist active exception takeover receipt");

        let mut metadata = ExceptionTakeoverMetadata {
            run_id: None,
            dispatch_target: None,
            dispatch_packet_path: None,
            source_exception_path_receipt_id: None,
            reason_class: "blocked_open_delegated_cycle_timeout".to_string(),
            active_bounded_unit: missing_exception_unit.to_string(),
            owned_write_scope: vec!["crates/vida/src".to_string()],
            why_delegated_or_rerouted_path_is_not_currently_lawful: "blocked".to_string(),
            why_local_write_is_the_smallest_safe_bounded_workaround: "bounded".to_string(),
            return_to_normal_posture_condition: "verified".to_string(),
            verification_plan: vec!["test".to_string()],
            recorded_at: "2026-06-05T00:00:00Z".to_string(),
        };
        metadata.bind_to_receipt(&receipt);
        write_exception_takeover_metadata(store.root(), run_id, &metadata)
            .expect("active exception metadata should persist");

        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: run_id.to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "run_graph_task",
                        "task_id": run_id,
                        "run_id": run_id,
                        "active_node": "implementer"
                    }),
                    binding_source: "test-active-exception-missing-unit".to_string(),
                    why_this_unit: "test active exception missing-unit stale run".to_string(),
                    primary_path: "exception_takeover_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
                    request_text: None,
                    recorded_at: "2026-06-05T00:00:01Z".to_string(),
                },
            )
            .await
            .expect("persist active exception continuation binding");
        let seeded_status = store
            .run_graph_status(run_id)
            .await
            .expect("read seeded status");
        let seeded_receipt = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read seeded receipt")
            .expect("receipt should exist");
        let seeded_binding = store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read seeded continuation binding")
            .expect("binding should remain present");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "retire".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                run_id.to_string(),
                "--reason".to_string(),
                "missing TaskFlow task stale run".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::from(2));

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after active exception retire rejection");
        let retained = store
            .run_graph_status(run_id)
            .await
            .expect("read retained status");
        assert_eq!(retained.status, seeded_status.status);
        assert_eq!(retained.lifecycle_stage, seeded_status.lifecycle_stage);
        assert_eq!(retained.resume_target, seeded_status.resume_target);
        assert_eq!(retained.recovery_ready, seeded_status.recovery_ready);
        let receipt = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read retained receipt")
            .expect("receipt should exist");
        assert_eq!(receipt.dispatch_status, seeded_receipt.dispatch_status);
        assert_eq!(receipt.lane_status, seeded_receipt.lane_status);
        assert_eq!(receipt.dispatch_target, seeded_receipt.dispatch_target);
        assert_eq!(
            receipt.dispatch_packet_path,
            seeded_receipt.dispatch_packet_path
        );
        let binding = store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read continuation binding")
            .expect("binding should remain present");
        assert_eq!(binding.status, seeded_binding.status);
        assert_eq!(binding.task_id, seeded_binding.task_id);
        assert_eq!(
            binding.active_bounded_unit,
            seeded_binding.active_bounded_unit
        );

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
        let mut metadata = ExceptionTakeoverMetadata {
            run_id: None,
            dispatch_target: None,
            dispatch_packet_path: None,
            source_exception_path_receipt_id: None,
            reason_class: "blocked_open_delegated_cycle_timeout".to_string(),
            active_bounded_unit: format!("{run_id}:spec-pack:exception-takeover"),
            owned_write_scope: vec!["crates/vida/src/lane_surface.rs".to_string()],
            why_delegated_or_rerouted_path_is_not_currently_lawful: "blocked".to_string(),
            why_local_write_is_the_smallest_safe_bounded_workaround: "bounded".to_string(),
            return_to_normal_posture_condition: "verified".to_string(),
            verification_plan: vec!["test".to_string()],
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };
        metadata.bind_to_receipt(&receipt);
        write_exception_takeover_metadata(store.root(), run_id, &metadata)
            .expect("metadata should persist");
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
        assert_eq!(advanced_status.lifecycle_stage, "implementer_complete");
        assert_eq!(advanced_status.handoff_state, "awaiting_coach");
        assert_eq!(advanced_status.resume_target, "dispatch.coach_lane");
        assert!(advanced_status.recovery_ready);
        assert_eq!(binding.binding_source, "lane_complete");
        assert_eq!(binding.active_bounded_unit["kind"], "run_graph_task");
        assert_eq!(binding.active_bounded_unit["active_node"], "implementer");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_complete_records_host_bridge_result_and_receipt_evidence() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-complete-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-host-bridge-complete";
        let task = store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge complete",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/lane_surface.rs".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let artifact_path = root.join("attempt-artifacts/host-bridge-complete.json");
        std::fs::create_dir_all(artifact_path.parent().expect("artifact parent"))
            .expect("create artifact parent");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "task_id": run_id,
                "stage_id": "implementation",
                "changed_files": ["crates/vida/src/lane_surface.rs"]
            })
            .to_string(),
        )
        .expect("write implementation artifact");
        store
            .record_task_attempt(crate::state_store::RecordTaskAttemptRequest {
                attempt_id: Some("host-bridge-complete-attempt".to_string()),
                task_id: run_id.to_string(),
                stage_id: "implementation".to_string(),
                backend: "internal_subagents".to_string(),
                model_profile: "middle".to_string(),
                isolation: "patch_proposal".to_string(),
                freshness: Some(task.updated_at.clone()),
                status: "accepted".to_string(),
                artifact_refs: vec![artifact_path.display().to_string()],
                consolidation_receipt_id: Some(
                    "host-bridge-complete-consolidation-receipt".to_string(),
                ),
                selected_model_profile_readiness_status: None,
                budget_posture: None,
                cap_posture: None,
                write_scope_classification: None,
            })
            .await
            .expect("record implementation attempt");
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.implementer".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_path =
            root.join("runtime-consumption/downstream-dispatch-packets/run-host-bridge.json");
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
                    "goal": "Complete host bridge lane evidence.",
                    "scope_in": ["dispatch_target:implementer"],
                    "handoff_task_class": "implementation",
                    "handoff_runtime_role": "worker",
                    "owned_paths": ["crates/vida/src/lane_surface.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption"],
                    "definition_of_done": ["host bridge completion is receipt-backed"],
                    "verification_command": "cargo test -p vida host_bridge",
                    "proof_target": "host bridge completion receipt",
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

        let request_path = root.join("host-tool-bridge/requests/run-host-bridge-implementer.json");
        let result_path = root.join("host-tool-bridge/results/run-host-bridge-implementer.json");
        let bridge_receipt_path =
            root.join("host-tool-bridge/receipts/run-host-bridge-implementer.json");
        std::fs::create_dir_all(
            request_path
                .parent()
                .expect("request path should have parent"),
        )
        .expect("create request dir");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "run-host-bridge-implementer",
                "run_id": run_id,
                "dispatch_target": "implementer",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "carrier_id": "junior",
                "execution_boundary": "parent_host_session",
                "dispatch_transport": "host_tool_bridge",
                "implementation_isolation": {
                    "schema_version": "implementation-isolation-v1",
                    "artifact_contract": "stage_attempt_implementation_artifact_v1",
                    "owned_paths": ["crates/vida/src/lane_surface.rs"]
                },
                "implementation_artifacts": [],
                "result_path": result_path.display().to_string(),
                "receipt_path": bridge_receipt_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write host bridge request");

        let activation_result_path =
            root.join("runtime-consumption/dispatch-results/run-host-bridge-activation.json");
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
                "status": "blocked",
                "execution_state": "bridge_request_pending",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "result_path": result_path.display().to_string(),
                    "receipt_path": bridge_receipt_path.display().to_string()
                }
            })
            .to_string(),
        )
        .expect("write activation result");

        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["pending_implementation_evidence".to_string()];
        receipt.downstream_dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_active_target = Some("implementer".to_string());
        receipt.selected_backend = Some("internal_subagents".to_string());
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
                "host-bridge-completion-1".to_string(),
                "--host-bridge-request".to_string(),
                request_path.display().to_string(),
                "--host-agent-id".to_string(),
                "agent-1".to_string(),
                "--host-bridge-summary".to_string(),
                "verifier proof passed focused host-bridge tests and confirmed pending receipt was the only closure blocker".to_string(),
                "--state-dir".to_string(),
                root.display().to_string(),
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
        assert_eq!(after.dispatch_status, "executed");
        assert_eq!(after.lane_status, crate::LaneStatus::LaneCompleted.as_str());
        assert!(after.downstream_dispatch_ready);
        assert!(after.downstream_dispatch_blockers.is_empty());
        let result_path_string = result_path.display().to_string();
        let bridge_receipt_path_string = bridge_receipt_path.display().to_string();
        assert_eq!(
            after.dispatch_result_path.as_deref(),
            Some(result_path_string.as_str())
        );
        assert_eq!(
            after.downstream_dispatch_trace_path.as_deref(),
            Some(bridge_receipt_path_string.as_str())
        );
        let dispatch_command = after
            .dispatch_command
            .as_deref()
            .expect("host bridge completion should persist replayable dispatch command");
        assert!(dispatch_command.contains("vida lane complete run-host-bridge-complete"));
        assert!(dispatch_command.contains("--receipt-id host-bridge-completion-1"));
        assert!(dispatch_command.contains("--host-bridge-request"));
        assert!(dispatch_command.contains("--host-agent-id agent-1"));
        assert!(dispatch_command.contains("--host-bridge-summary"));
        assert!(dispatch_command.contains("pending receipt was the only closure blocker"));
        assert!(dispatch_command.contains("--state-dir"));
        assert!(dispatch_command.contains("--json"));
        let bridge_result: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&result_path).expect("read host bridge result"),
        )
        .expect("host bridge result should be json");
        assert_eq!(bridge_result["artifact_kind"], "host_tool_bridge_result");
        assert_eq!(bridge_result["execution_evidence"]["receipt_backed"], true);
        let bridge_receipt: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&bridge_receipt_path).expect("read host bridge receipt"),
        )
        .expect("host bridge receipt should be json");
        assert_eq!(bridge_receipt["artifact_kind"], "host_tool_bridge_receipt");
        assert_eq!(bridge_receipt["receipt_backed"], true);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_complete_host_bridge_reconciles_retryable_stale_result_schema() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-stale-result-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-host-bridge-stale-result";
        let task = store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge stale result reconciliation",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/lane_surface.rs".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let artifact_path = root.join("attempt-artifacts/stale-result-implementation.json");
        std::fs::create_dir_all(artifact_path.parent().expect("artifact parent"))
            .expect("create artifact parent");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "task_id": run_id,
                "stage_id": "implementation",
                "changed_files": ["crates/vida/src/lane_surface.rs"]
            })
            .to_string(),
        )
        .expect("write implementation artifact");
        store
            .record_task_attempt(crate::state_store::RecordTaskAttemptRequest {
                attempt_id: Some("stale-result-attempt-1".to_string()),
                task_id: run_id.to_string(),
                stage_id: "implementation".to_string(),
                backend: "internal_subagents".to_string(),
                model_profile: "middle".to_string(),
                isolation: "patch_proposal".to_string(),
                freshness: Some(task.updated_at.clone()),
                status: "accepted".to_string(),
                artifact_refs: vec![artifact_path.display().to_string()],
                consolidation_receipt_id: Some("stale-result-consolidation-receipt".to_string()),
                selected_model_profile_readiness_status: None,
                budget_posture: None,
                cap_posture: None,
                write_scope_classification: None,
            })
            .await
            .expect("record implementation attempt");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.implementer".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_path = root.join(
            "runtime-consumption/downstream-dispatch-packets/run-host-bridge-stale-result.json",
        );
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
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
                    "goal": "Complete host bridge lane evidence.",
                    "scope_in": ["dispatch_target:implementer"],
                    "handoff_task_class": "implementation",
                    "handoff_runtime_role": "worker",
                    "owned_paths": ["crates/vida/src/lane_surface.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption"],
                    "definition_of_done": ["host bridge completion reconciles stale result schema"],
                    "verification_command": "cargo test -p vida lane_complete_host_bridge_reconciles_retryable_stale_result_schema",
                    "proof_target": "host bridge stale result reconciliation",
                    "stop_rules": ["stop if bridge evidence is missing"],
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
        .expect("write packet");

        let request_path = root.join("host-tool-bridge/requests/run-host-bridge-stale-result.json");
        let result_path = root.join("host-tool-bridge/results/run-host-bridge-stale-result.json");
        let bridge_receipt_path =
            root.join("host-tool-bridge/receipts/run-host-bridge-stale-result.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("create result parent");
        std::fs::create_dir_all(bridge_receipt_path.parent().expect("receipt parent"))
            .expect("create receipt parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "retryable_blocked",
                "request_id": "run-host-bridge-stale-result",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "implementer",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "carrier_id": "junior",
                "execution_boundary": "parent_host_session",
                "dispatch_transport": "host_tool_bridge",
                "implementation_isolation": {
                    "schema_version": "implementation-isolation-v1",
                    "artifact_contract": "stage_attempt_implementation_artifact_v1",
                    "owned_paths": ["crates/vida/src/lane_surface.rs"]
                },
                "implementation_artifacts": [{
                    "artifact_kind": "patch_proposal",
                    "attempt_id": "stale-result-attempt-1",
                    "task_id": run_id,
                    "stage_id": "implementation",
                    "freshness": task.updated_at,
                    "receipt_backed": true,
                    "consolidation_receipt_id": "stale-result-consolidation-receipt",
                    "changed_files": ["crates/vida/src/lane_surface.rs"]
                }],
                "result_path": result_path.display().to_string(),
                "receipt_path": bridge_receipt_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write host bridge request");

        let activation_result_path = root.join(
            "runtime-consumption/dispatch-results/run-host-bridge-stale-result-activation.json",
        );
        std::fs::create_dir_all(activation_result_path.parent().expect("activation parent"))
            .expect("create activation parent");
        std::fs::write(
            &activation_result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "schema_version": 1,
                "status": "blocked",
                "execution_state": "blocked",
                "request_id": "run-host-bridge-stale-result",
                "run_id": run_id,
                "dispatch_target": "implementer",
                "blocker_code": "implementation_artifacts_missing",
                "blocker_codes": ["implementation_artifacts_missing"]
            })
            .to_string(),
        )
        .expect("write stale activation result");
        std::fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "schema_version": 1,
                "status": "blocked",
                "execution_state": "blocked",
                "request_id": "run-host-bridge-stale-result",
                "run_id": run_id,
                "dispatch_target": "implementer",
                "blocker_code": "implementation_artifacts_missing",
                "blocker_codes": ["implementation_artifacts_missing"]
            })
            .to_string(),
        )
        .expect("write stale host bridge result");

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());
        receipt.blocker_code = Some("implementation_artifacts_missing".to_string());
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["implementation_artifacts_missing".to_string()];
        receipt.downstream_dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_active_target = Some("implementer".to_string());
        receipt.selected_backend = Some("internal_subagents".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist stale dispatch receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "complete".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "host-bridge-stale-result-completion".to_string(),
                "--host-bridge-request".to_string(),
                request_path.display().to_string(),
                "--host-agent-id".to_string(),
                "agent-1".to_string(),
                "--host-bridge-summary".to_string(),
                "internal agent completed".to_string(),
                "--state-dir".to_string(),
                root.display().to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::SUCCESS);

        let bridge_result: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&result_path).expect("read reconciled result"),
        )
        .expect("result should be json");
        assert_eq!(bridge_result["status"], "pass");
        let recorded_request_path = bridge_result["host_tool_bridge_request"]["request_path"]
            .as_str()
            .expect("recorded request path should be a string");
        assert!(recorded_request_path.ends_with("run-host-bridge-stale-result.json"));
        assert_eq!(bridge_result["scope_validation"]["status"], "pass");

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after stale result retry");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt after retry")
            .expect("receipt should exist");
        assert_eq!(after.dispatch_status, "executed");
        assert_eq!(after.lane_status, crate::LaneStatus::LaneCompleted.as_str());
        assert!(after
            .dispatch_result_path
            .as_deref()
            .is_some_and(|path| path.ends_with("run-host-bridge-stale-result.json")));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_complete_host_bridge_summary_guard_can_retry_with_corrected_summary() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-summary-retry-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-host-bridge-summary-retry";
        let task = store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge summary retry",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/lane_surface.rs".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let artifact_path = root.join("attempt-artifacts/summary-attempt.json");
        std::fs::create_dir_all(artifact_path.parent().expect("artifact parent"))
            .expect("create artifact parent");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "task_id": run_id,
                "stage_id": "implementation",
                "changed_files": ["crates/vida/src/lane_surface.rs"]
            })
            .to_string(),
        )
        .expect("write summary attempt artifact");
        store
            .record_task_attempt(crate::state_store::RecordTaskAttemptRequest {
                attempt_id: Some("summary-attempt-1".to_string()),
                task_id: run_id.to_string(),
                stage_id: "implementation".to_string(),
                backend: "internal_subagents".to_string(),
                model_profile: "middle".to_string(),
                isolation: "patch_proposal".to_string(),
                freshness: None,
                status: "accepted".to_string(),
                artifact_refs: vec![artifact_path.display().to_string()],
                consolidation_receipt_id: Some("summary-consolidation-receipt".to_string()),
                selected_model_profile_readiness_status: None,
                budget_posture: None,
                cap_posture: None,
                write_scope_classification: None,
            })
            .await
            .expect("record summary attempt");
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.implementer".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_path = root.join(
            "runtime-consumption/downstream-dispatch-packets/run-host-bridge-summary-retry.json",
        );
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
                    "goal": "Complete host bridge lane evidence.",
                    "scope_in": ["dispatch_target:implementer"],
                    "handoff_task_class": "implementation",
                    "handoff_runtime_role": "worker",
                    "owned_paths": ["crates/vida/src/lane_surface.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption"],
                    "definition_of_done": ["host bridge evidence is recorded"],
                    "verification_command": "cargo test -p vida lane_complete",
                    "proof_target": "host bridge completion receipt",
                    "stop_rules": ["stop if bridge evidence is missing"],
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

        let request_path =
            root.join("host-tool-bridge/requests/run-host-bridge-summary-retry.json");
        let result_path = root.join("host-tool-bridge/results/run-host-bridge-summary-retry.json");
        let bridge_receipt_path =
            root.join("host-tool-bridge/receipts/run-host-bridge-summary-retry.json");
        std::fs::create_dir_all(
            request_path
                .parent()
                .expect("request path should have parent"),
        )
        .expect("create request dir");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "run-host-bridge-summary-retry",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "implementer",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "carrier_id": "junior",
                "execution_boundary": "parent_host_session",
                "dispatch_transport": "host_tool_bridge",
                "implementation_isolation": {
                    "schema_version": "implementation-isolation-v1",
                    "artifact_contract": "stage_attempt_implementation_artifact_v1",
                    "owned_paths": ["crates/vida/src/lane_surface.rs"]
                },
                "implementation_artifacts": [{
                    "artifact_kind": "patch_proposal",
                    "attempt_id": "summary-attempt-1",
                    "task_id": run_id,
                    "stage_id": "implementation",
                    "freshness": task.updated_at,
                    "receipt_backed": true,
                    "consolidation_receipt_id": "summary-consolidation-receipt",
                    "changed_files": ["crates/vida/src/lane_surface.rs"]
                }],
                "result_path": result_path.display().to_string(),
                "receipt_path": bridge_receipt_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write host bridge request");

        let activation_result_path = root.join(
            "runtime-consumption/dispatch-results/run-host-bridge-summary-retry-activation.json",
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
                "status": "blocked",
                "execution_state": "bridge_request_pending",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "result_path": result_path.display().to_string(),
                    "receipt_path": bridge_receipt_path.display().to_string()
                }
            })
            .to_string(),
        )
        .expect("write activation result");

        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["pending_implementation_evidence".to_string()];
        receipt.downstream_dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_active_target = Some("implementer".to_string());
        receipt.selected_backend = Some("internal_subagents".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let blocked_args = ProxyArgs {
            args: vec![
                "complete".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "host-bridge-summary-blocked".to_string(),
                "--host-bridge-request".to_string(),
                request_path.display().to_string(),
                "--host-agent-id".to_string(),
                "agent-1".to_string(),
                "--host-bridge-summary".to_string(),
                "verdict: blocker; read-only host evidence blocked by explicit rework wording"
                    .to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(blocked_args).await, ExitCode::from(2));

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after blocked lane command");
        let mut blocked = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read blocked receipt")
            .expect("receipt should exist");
        assert_eq!(blocked.dispatch_status, "blocked");
        assert_eq!(
            blocked.blocker_code.as_deref(),
            Some("lane_completion_blocked_by_summary")
        );
        blocked.blocker_code = None;
        store
            .record_run_graph_dispatch_receipt(&blocked)
            .await
            .expect("persist downstream-only summary blocker receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let retry_args = ProxyArgs {
            args: vec![
                "complete".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "host-bridge-summary-retry".to_string(),
                "--host-bridge-request".to_string(),
                request_path.display().to_string(),
                "--host-agent-id".to_string(),
                "agent-1".to_string(),
                "--host-bridge-summary".to_string(),
                "internal agent completed".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(retry_args).await, ExitCode::SUCCESS);

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after retry");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt after")
            .expect("receipt should exist");
        assert_eq!(after.dispatch_status, "executed");
        assert_eq!(after.lane_status, crate::LaneStatus::LaneCompleted.as_str());
        assert!(after.downstream_dispatch_ready);
        assert!(after.downstream_dispatch_blockers.is_empty());
        assert!(after
            .dispatch_result_path
            .as_deref()
            .is_some_and(|path| path.ends_with("run-host-bridge-summary-retry.json")));

        let bridge_result: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&result_path).expect("read retried host bridge result"),
        )
        .expect("host bridge result should be json");
        assert_eq!(bridge_result["status"], "pass");
        assert_eq!(bridge_result["execution_state"], "executed");
        assert_eq!(
            bridge_result["completion_receipt_id"],
            "host-bridge-summary-retry"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_complete_host_bridge_pending_receipt_requires_request_path() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-missing-request-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-host-bridge-missing-request";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.implementer".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_path =
            root.join("runtime-consumption/downstream-dispatch-packets/run-missing-request.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": run_id,
                "dispatch_target": "implementer",
                "activation_runtime_role": "worker",
                "packet_template_kind": "delivery_task_packet",
                "owned_paths": ["crates/vida/src/lib.rs"],
                "read_only_paths": ["crates/vida/src"],
                "delivery_task_packet": {
                    "goal": "Complete host bridge lane evidence.",
                    "scope_in": ["dispatch_target:implementer"],
                    "handoff_task_class": "implementation",
                    "handoff_runtime_role": "worker",
                    "owned_paths": ["crates/vida/src/lib.rs"],
                    "read_only_paths": ["crates/vida/src"],
                    "definition_of_done": ["host bridge request evidence is required"],
                    "verification_command": "cargo test -p vida lane_complete_host_bridge_pending_receipt_requires_request_path",
                    "proof_target": "host bridge completion receipt",
                    "stop_rules": ["stop if bridge evidence is missing"],
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
        .expect("write packet");

        let activation_result_path =
            root.join("runtime-consumption/dispatch-results/run-missing-request-activation.json");
        std::fs::create_dir_all(activation_result_path.parent().expect("activation parent"))
            .expect("create activation parent");
        std::fs::write(
            &activation_result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "bridge_request_pending"
            })
            .to_string(),
        )
        .expect("write activation result");
        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
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
                "host-bridge-missing-request".to_string(),
                "--host-agent-id".to_string(),
                "agent-1".to_string(),
                "--host-bridge-summary".to_string(),
                "internal agent completed".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::from(2));

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt")
            .expect("receipt should remain");
        assert_eq!(after.dispatch_status, "bridge_request_pending");
        assert!(!after.downstream_dispatch_ready);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_complete_host_bridge_accepts_taskflow_attempt_artifacts_when_request_is_empty() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-attempt-artifacts-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-host-bridge-attempt-artifacts";
        let owned_paths = vec!["crates/vida/src/lib.rs".to_string()];

        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge attempt artifact completion",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: owned_paths.clone(),
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");

        let artifact_path = root.join("attempt-artifacts/attempt-1.json");
        std::fs::create_dir_all(artifact_path.parent().expect("artifact parent"))
            .expect("create artifact parent");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "task_handoff_accept_receipt",
                "task_id": run_id,
                "changed_files": ["crates/vida/src/lib.rs"],
                "proof_commands": ["cargo test -p vida lane_complete_host_bridge_accepts_taskflow_attempt_artifacts_when_request_is_empty"]
            })
            .to_string(),
        )
        .expect("write attempt artifact");
        store
            .record_task_attempt(crate::state_store::RecordTaskAttemptRequest {
                attempt_id: Some("attempt-1".to_string()),
                task_id: run_id.to_string(),
                stage_id: "implementation".to_string(),
                backend: "internal_subagents".to_string(),
                model_profile: "middle".to_string(),
                isolation: "patch_proposal".to_string(),
                freshness: None,
                status: "accepted".to_string(),
                artifact_refs: vec![artifact_path.display().to_string()],
                consolidation_receipt_id: Some("attempt-1-consolidation-receipt".to_string()),
                selected_model_profile_readiness_status: None,
                budget_posture: None,
                cap_posture: None,
                write_scope_classification: None,
            })
            .await
            .expect("record accepted attempt");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.implementer".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_path =
            root.join("runtime-consumption/downstream-dispatch-packets/run-attempt-artifacts.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": run_id,
                "dispatch_target": "implementer",
                "activation_runtime_role": "worker",
                "packet_template_kind": "delivery_task_packet",
                "owned_paths": owned_paths,
                "read_only_paths": ["crates/vida/src"],
                "delivery_task_packet": {
                    "goal": "Complete host bridge lane evidence.",
                    "scope_in": ["dispatch_target:implementer"],
                    "handoff_task_class": "implementation",
                    "handoff_runtime_role": "worker",
                    "owned_paths": ["crates/vida/src/lib.rs"],
                    "read_only_paths": ["crates/vida/src"],
                    "definition_of_done": ["attempt artifact evidence is accepted"],
                    "verification_command": "cargo test -p vida lane_complete_host_bridge_accepts_taskflow_attempt_artifacts_when_request_is_empty",
                    "proof_target": "host bridge completion receipt",
                    "stop_rules": ["stop if bridge evidence is missing"],
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
        .expect("write packet");

        let request_path = root.join("host-tool-bridge/requests/run-attempt-artifacts.json");
        let result_path = root.join("host-tool-bridge/results/run-attempt-artifacts.json");
        let bridge_receipt_path = root.join("host-tool-bridge/receipts/run-attempt-artifacts.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "run-attempt-artifacts",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "implementer",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "carrier_id": "middle",
                "execution_boundary": "parent_host_session",
                "dispatch_transport": "host_tool_bridge",
                "implementation_isolation": {
                    "schema_version": "implementation-isolation-v1",
                    "artifact_contract": "stage_attempt_implementation_artifact_v1",
                    "owned_paths": ["crates/vida/src/lib.rs"]
                },
                "implementation_artifacts": [],
                "result_path": result_path.display().to_string(),
                "receipt_path": bridge_receipt_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write request");
        let activation_result_path =
            root.join("runtime-consumption/dispatch-results/run-attempt-artifacts-activation.json");
        std::fs::create_dir_all(activation_result_path.parent().expect("activation parent"))
            .expect("create activation parent");
        std::fs::write(
            &activation_result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "bridge_request_pending",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "result_path": result_path.display().to_string(),
                    "receipt_path": bridge_receipt_path.display().to_string()
                }
            })
            .to_string(),
        )
        .expect("write activation result");
        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["pending_implementation_evidence".to_string()];
        receipt.downstream_dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_active_target = Some("implementer".to_string());
        receipt.selected_backend = Some("internal_subagents".to_string());
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
                "host-bridge-attempt-artifacts".to_string(),
                "--host-bridge-request".to_string(),
                request_path.display().to_string(),
                "--host-agent-id".to_string(),
                "agent-1".to_string(),
                "--host-bridge-summary".to_string(),
                "internal agent completed".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::SUCCESS);

        let bridge_result: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&result_path).expect("read host bridge result"),
        )
        .expect("host bridge result should be json");
        assert_eq!(bridge_result["status"], "pass");
        assert_eq!(
            bridge_result["implementation_artifact_source"],
            "taskflow_attempt_ledger"
        );
        assert_eq!(bridge_result["scope_validation"]["status"], "pass");
        assert_eq!(
            bridge_result["scope_validation"]["reported_changed_files"],
            serde_json::json!(["crates/vida/src/lib.rs"])
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    async fn collect_attempt_artifacts_for_empty_host_bridge_request(
        test_name: &str,
        run_id: &str,
        stage_id: &str,
        freshness: Option<String>,
        consolidation_receipt_id: Option<String>,
    ) -> crate::runtime_dispatch_packets::TaskflowImplementationArtifacts {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-{test_name}-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge attempt artifact selection",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/lib.rs".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let artifact_path = root.join("attempt-artifacts/attempt.json");
        std::fs::create_dir_all(artifact_path.parent().expect("artifact parent"))
            .expect("create artifact parent");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "task_id": run_id,
                "changed_files": ["crates/vida/src/lib.rs"]
            })
            .to_string(),
        )
        .expect("write attempt artifact");
        store
            .record_task_attempt(crate::state_store::RecordTaskAttemptRequest {
                attempt_id: Some(format!("{test_name}-attempt")),
                task_id: run_id.to_string(),
                stage_id: stage_id.to_string(),
                backend: "internal_subagents".to_string(),
                model_profile: "middle".to_string(),
                isolation: "patch_proposal".to_string(),
                freshness,
                status: "accepted".to_string(),
                artifact_refs: vec![artifact_path.display().to_string()],
                consolidation_receipt_id,
                selected_model_profile_readiness_status: None,
                budget_posture: None,
                cap_posture: None,
                write_scope_classification: None,
            })
            .await
            .expect("record accepted attempt");

        let request_path = root.join("host-tool-bridge/requests/request.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": test_name,
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "implementer",
                "implementation_artifacts": []
            })
            .to_string(),
        )
        .expect("write request");
        let artifacts = taskflow_implementation_artifacts_for_host_bridge_request(
            &store,
            &request_path.display().to_string(),
            run_id,
        )
        .await;
        let _ = std::fs::remove_dir_all(&root);
        artifacts.taskflow_artifacts
    }

    #[tokio::test]
    async fn host_bridge_attempt_fallback_rejects_non_implementation_stage_attempts() {
        let _guard = acquire_lane_surface_test_lock();
        let artifacts = collect_attempt_artifacts_for_empty_host_bridge_request(
            "host-bridge-non-implementation-attempt",
            "run-host-bridge-non-implementation-attempt",
            "coach",
            None,
            Some("non-implementation-consolidation-receipt".to_string()),
        )
        .await;
        assert!(artifacts.artifacts.is_empty());
        assert!(artifacts.artifact_refs.is_empty());
    }

    #[tokio::test]
    async fn host_bridge_attempt_fallback_rejects_stale_implementation_attempts() {
        let _guard = acquire_lane_surface_test_lock();
        let artifacts = collect_attempt_artifacts_for_empty_host_bridge_request(
            "host-bridge-stale-implementation-attempt",
            "run-host-bridge-stale-implementation-attempt",
            "implementation",
            Some("stale-task-updated-at".to_string()),
            Some("stale-implementation-consolidation-receipt".to_string()),
        )
        .await;
        assert!(artifacts.artifacts.is_empty());
        assert!(artifacts.artifact_refs.is_empty());
    }

    #[tokio::test]
    async fn host_bridge_attempt_fallback_rejects_receiptless_implementation_attempts() {
        let _guard = acquire_lane_surface_test_lock();
        let artifacts = collect_attempt_artifacts_for_empty_host_bridge_request(
            "host-bridge-receiptless-implementation-attempt",
            "run-host-bridge-receiptless-implementation-attempt",
            "implementation",
            None,
            None,
        )
        .await;
        assert!(artifacts.artifacts.is_empty());
        assert!(artifacts.artifact_refs.is_empty());
    }

    #[tokio::test]
    async fn host_bridge_attempt_fallback_rejects_request_task_rebinding() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-request-rebind-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-host-bridge-request-rebind";
        for task_id in [run_id, "other-task"] {
            store
                .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                    task_id,
                    title: "Host bridge request task authority",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "open",
                    priority: 1,
                    parent_id: None,
                    labels: &[],
                    execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create task");
        }
        let request_path = root.join("host-tool-bridge/requests/rebind.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "rebind",
                "run_id": run_id,
                "task_id": "other-task",
                "dispatch_target": "implementer",
                "implementation_artifacts": []
            })
            .to_string(),
        )
        .expect("write request");

        let evidence = taskflow_implementation_artifacts_for_host_bridge_request(
            &store,
            &request_path.display().to_string(),
            run_id,
        )
        .await;

        assert!(evidence.authority.is_none());
        assert!(evidence.taskflow_artifacts.artifacts.is_empty());
        assert_eq!(
            evidence.blocker_codes,
            vec!["host_bridge_request_task_mismatch".to_string()]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    fn read_host_bridge_request_quickly(
        state_root: std::path::PathBuf,
        request_path: std::path::PathBuf,
    ) -> Result<serde_json::Value, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let request_path = request_path
                .to_str()
                .expect("host bridge request path should be UTF-8")
                .to_string();
            let result = read_host_bridge_request(&state_root, &request_path);
            let _ = tx.send(result);
        });
        rx.recv_timeout(std::time::Duration::from_secs(2))
            .expect("host bridge request read should return quickly")
    }

    #[test]
    fn host_bridge_request_rejects_out_of_root_or_oversized_file() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-request-read-{pid}-{nanos}"
        ));
        std::fs::create_dir_all(root.join("host-tool-bridge/requests"))
            .expect("create request root");

        let outside_root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-request-outside-{pid}-{nanos}"
        ));
        let outside_request_path = outside_root.join("host-tool-bridge/requests/outside.json");
        std::fs::create_dir_all(outside_request_path.parent().expect("outside parent"))
            .expect("create outside request parent");
        std::fs::write(
            &outside_request_path,
            serde_json::json!({
                "request_id": "outside-root",
                "status": "pending"
            })
            .to_string(),
        )
        .expect("write outside request");
        let outside_error =
            read_host_bridge_request_quickly(root.clone(), outside_request_path.clone())
                .expect_err("outside-root request should fail");
        assert!(
            outside_error.contains("outside VIDA state root"),
            "{outside_error}"
        );

        let dot_segment_request_path = root.join("host-tool-bridge/requests/dot-segment.json");
        std::fs::write(
            &dot_segment_request_path,
            serde_json::json!({
                "request_id": "dot-segment",
                "status": "pending"
            })
            .to_string(),
        )
        .expect("write dot-segment request");
        let dot_segment_path = root
            .join("host-tool-bridge")
            .join("requests")
            .join(".")
            .join("dot-segment.json");
        let dot_segment_error = read_host_bridge_request_quickly(root.clone(), dot_segment_path)
            .expect_err("dot-segment request should fail");
        assert!(
            dot_segment_error.contains("dot-segment"),
            "{dot_segment_error}"
        );

        let oversized_request_path = root.join("host-tool-bridge/requests/oversized.json");
        let oversized_request = serde_json::json!({
            "request_id": "oversized",
            "status": "pending",
            "pad": "x".repeat((MAX_HOST_BRIDGE_REQUEST_BYTES as usize) + 1),
        });
        std::fs::write(
            &oversized_request_path,
            serde_json::to_vec(&oversized_request).expect("serialize oversized request"),
        )
        .expect("write oversized request");
        let oversized_error =
            read_host_bridge_request_quickly(root.clone(), oversized_request_path)
                .expect_err("oversized request should fail");
        assert!(oversized_error.contains("exceeds"), "{oversized_error}");
        assert!(
            oversized_error.contains(&MAX_HOST_BRIDGE_REQUEST_BYTES.to_string()),
            "{oversized_error}"
        );

        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::os::unix::ffi::OsStrExt;

            let fifo_request_path = root.join("host-tool-bridge/requests/fifo.json");
            let fifo_c_path = CString::new(fifo_request_path.as_os_str().as_bytes())
                .expect("fifo path should be a valid C string");
            let fifo_result = unsafe { libc::mkfifo(fifo_c_path.as_ptr(), 0o644) };
            assert_eq!(
                fifo_result,
                0,
                "mkfifo failed: {}",
                std::io::Error::last_os_error()
            );
            let fifo_error = read_host_bridge_request_quickly(root.clone(), fifo_request_path)
                .expect_err("fifo request should fail");
            assert!(fifo_error.contains("regular file"), "{fifo_error}");

            let symlink_request_path = root.join("host-tool-bridge/requests/symlink.json");
            std::os::unix::fs::symlink(&outside_request_path, &symlink_request_path)
                .expect("create symlink request");
            let symlink_error =
                read_host_bridge_request_quickly(root.clone(), symlink_request_path)
                    .expect_err("symlink request should fail");
            assert!(symlink_error.contains("regular file"), "{symlink_error}");
        }

        let _ = std::fs::remove_dir_all(&outside_root);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn read_lane_packet_reads_contained_packet_and_rejects_traversal_symlink_and_oversized_file() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();
        let root =
            std::env::temp_dir().join(format!("vida-lane-surface-lane-packet-read-{pid}-{nanos}"));
        std::fs::create_dir_all(&root).expect("create packet root");

        let packet_path = root.join("runtime-consumption/packets/packet.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": "run-contained",
                "delivery_task_packet": { "task_id": "task-contained" }
            })
            .to_string(),
        )
        .expect("write packet");
        let packet = read_lane_packet(&root, packet_path.to_str().expect("utf-8"))
            .expect("contained packet should read");
        assert_eq!(packet["run_id"], "run-contained");
        assert_eq!(packet["delivery_task_packet"]["task_id"], "task-contained");

        let outside_root = std::env::temp_dir().join(format!(
            "vida-lane-surface-lane-packet-outside-{pid}-{nanos}"
        ));
        let outside_packet_path = outside_root.join("runtime-consumption/packets/outside.json");
        std::fs::create_dir_all(outside_packet_path.parent().expect("outside parent"))
            .expect("create outside parent");
        std::fs::write(
            &outside_packet_path,
            serde_json::json!({
                "run_id": "run-outside",
                "delivery_task_packet": { "task_id": "task-outside" }
            })
            .to_string(),
        )
        .expect("write outside packet");
        let outside_error = read_lane_packet(&root, outside_packet_path.to_str().expect("utf-8"))
            .expect_err("outside-root packet should fail");
        assert!(
            outside_error.contains("outside VIDA state root"),
            "{outside_error}"
        );

        let dot_segment_path = root
            .join("runtime-consumption")
            .join("packets")
            .join(".")
            .join("dot-segment.json");
        let dot_segment_error = read_lane_packet(&root, dot_segment_path.to_str().expect("utf-8"))
            .expect_err("dot-segment packet should fail");
        assert!(
            dot_segment_error.contains("dot-segment"),
            "{dot_segment_error}"
        );

        let oversized_path = root.join("runtime-consumption/packets/oversized.json");
        std::fs::write(
            &oversized_path,
            serde_json::json!({
                "run_id": "run-oversized",
                "payload": "x".repeat((MAX_LANE_PACKET_READ_BYTES as usize) + 1)
            })
            .to_string(),
        )
        .expect("write oversized packet");
        let oversized_error = read_lane_packet(&root, oversized_path.to_str().expect("utf-8"))
            .expect_err("oversized packet should fail");
        assert!(oversized_error.contains("exceeds"), "{oversized_error}");

        #[cfg(unix)]
        {
            let symlink_path = root.join("runtime-consumption/packets/symlink.json");
            let symlink_target = root.join("runtime-consumption/packets/packet.json");
            std::os::unix::fs::symlink(&symlink_target, &symlink_path)
                .expect("create symlink packet");
            let symlink_error = read_lane_packet(&root, symlink_path.to_str().expect("utf-8"))
                .expect_err("symlink packet should fail");
            assert!(symlink_error.contains("regular file"), "{symlink_error}");
        }

        let _ = std::fs::remove_dir_all(&outside_root);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_attempt_fallback_blocks_missing_active_task_authority() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-missing-authority-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-host-bridge-missing-authority";
        let request_path = root.join("host-tool-bridge/requests/missing-authority.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "missing-authority",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "implementer",
                "implementation_artifacts": []
            })
            .to_string(),
        )
        .expect("write request");

        let evidence = taskflow_implementation_artifacts_for_host_bridge_request(
            &store,
            &request_path.display().to_string(),
            run_id,
        )
        .await;

        assert!(evidence.authority.is_none());
        assert!(evidence.taskflow_artifacts.artifacts.is_empty());
        assert_eq!(
            evidence.blocker_codes,
            vec!["implementation_artifact_authority_missing".to_string()]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_taskflow_implementation_artifacts_blocks_invalid_artifact_evidence() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-invalid-artifact-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-host-bridge-invalid-artifact";
        let _task = store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge invalid artifact",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/lane_surface.rs".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");

        let artifact_path = root.join("attempt-artifacts/invalid-artifact.json");
        std::fs::create_dir_all(artifact_path.parent().expect("artifact parent"))
            .expect("create artifact parent");
        std::fs::write(&artifact_path, "not json").expect("write invalid artifact");
        store
            .record_task_attempt(crate::state_store::RecordTaskAttemptRequest {
                attempt_id: Some("attempt-invalid".to_string()),
                task_id: run_id.to_string(),
                stage_id: "implementation".to_string(),
                backend: "internal_subagents".to_string(),
                model_profile: "middle".to_string(),
                isolation: "patch_proposal".to_string(),
                freshness: None,
                status: "accepted".to_string(),
                artifact_refs: vec![artifact_path.display().to_string()],
                consolidation_receipt_id: Some("attempt-invalid-consolidation-receipt".to_string()),
                selected_model_profile_readiness_status: None,
                budget_posture: None,
                cap_posture: None,
                write_scope_classification: None,
            })
            .await
            .expect("record accepted attempt");

        let request_path = root.join("host-tool-bridge/requests/invalid-artifact.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "invalid-artifact",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "implementer",
                "implementation_artifacts": []
            })
            .to_string(),
        )
        .expect("write request");

        let evidence = taskflow_implementation_artifacts_for_host_bridge_request(
            &store,
            &request_path.display().to_string(),
            run_id,
        )
        .await;

        assert_eq!(
            evidence.blocker_codes,
            vec!["implementation_artifact_contract_invalid".to_string()]
        );
        assert!(evidence.taskflow_artifacts.artifacts.is_empty());
        assert!(evidence.taskflow_artifacts.artifact_refs.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_scope_validation_blocks_missing_implementation_isolation() {
        let validation = host_bridge_implementation_scope_validation(
            &serde_json::json!({
                "run_id": "run-host-bridge-missing-isolation",
                "task_id": "run-host-bridge-missing-isolation",
                "dispatch_target": "implementer",
                "owned_paths": ["crates/vida/src/lib.rs"]
            }),
            &serde_json::json!([{
                "artifact_kind": "patch_proposal",
                "attempt_id": "attempt-1",
                "task_id": "run-host-bridge-missing-isolation",
                "stage_id": "implementation",
                "freshness": "updated-at-1",
                "receipt_backed": true,
                "consolidation_receipt_id": "receipt-1",
                "changed_files": ["crates/vida/src/lib.rs"]
            }]),
            crate::runtime_dispatch_packets::ImplementationArtifactAuthority {
                task_id: "run-host-bridge-missing-isolation",
                task_updated_at: "updated-at-1",
            },
            &["crates/vida/src/lib.rs".to_string()],
        );

        assert_eq!(validation["status"], "blocked");
        assert!(validation["blocker_codes"]
            .as_array()
            .expect("blocker codes")
            .iter()
            .any(|code| code == "implementation_artifact_contract_invalid"));
    }

    #[test]
    fn retryable_host_bridge_completion_requires_receipt_bound_packet() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-receipt-bound-packet-{}-{}",
            std::process::id(),
            nanos
        ));
        let receipt_packet_path =
            root.join("runtime-consumption/downstream-dispatch-packets/receipt-bound.json");
        let request_packet_path =
            root.join("runtime-consumption/downstream-dispatch-packets/request-forged.json");
        std::fs::create_dir_all(receipt_packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        for packet_path in [&receipt_packet_path, &request_packet_path] {
            std::fs::write(
                packet_path,
                serde_json::json!({
                    "run_id": "run-host-bridge-retryable-receipt-bound",
                    "dispatch_target": "implementer",
                    "downstream_dispatch_active_target": "implementer",
                    "downstream_dispatch_status": "blocked"
                })
                .to_string(),
            )
            .expect("write packet");
        }
        let request_path = root.join("host-tool-bridge/requests/retryable-forged.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "retryable_blocked",
                "request_id": "retryable-forged",
                "run_id": "run-host-bridge-retryable-receipt-bound",
                "task_id": "run-host-bridge-retryable-receipt-bound",
                "dispatch_target": "implementer",
                "packet_path": request_packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "dispatch_transport": "host_tool_bridge"
            })
            .to_string(),
        )
        .expect("write request");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-host-bridge-retryable-receipt-bound",
            "implementation",
            "implementation",
        );
        status.active_node = "implementer".to_string();
        status.status = "blocked".to_string();

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = "run-host-bridge-retryable-receipt-bound".to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.downstream_dispatch_packet_path = Some(receipt_packet_path.display().to_string());

        let error = match trusted_host_bridge_completion_request_context(
            &root,
            "run-host-bridge-retryable-receipt-bound",
            &request_path.display().to_string(),
            Some(&status),
            &receipt,
        ) {
            Err(error) => error,
            Ok(_) => panic!("forged retryable packet must be rejected"),
        };

        assert!(error.contains(
            "Host bridge request packet path does not match persisted dispatch receipt evidence"
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pending_host_bridge_completion_allows_original_receipt_packet_without_downstream_packet() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-pending-original-packet-{}-{}",
            std::process::id(),
            nanos
        ));
        let packet_path =
            root.join("runtime-consumption/dispatch-packets/pending-original-packet.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": "run-host-bridge-pending-original-packet",
                "dispatch_target": "implementer",
                "downstream_dispatch_active_target": "implementer",
                "downstream_dispatch_status": "blocked"
            })
            .to_string(),
        )
        .expect("write packet");
        let request_path = root.join("host-tool-bridge/requests/pending-original.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "bridge_request_pending",
                "request_id": "pending-original",
                "run_id": "run-host-bridge-pending-original-packet",
                "task_id": "run-host-bridge-pending-original-packet",
                "dispatch_target": "implementer",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "dispatch_transport": "host_tool_bridge"
            })
            .to_string(),
        )
        .expect("write request");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-host-bridge-pending-original-packet",
            "implementation",
            "implementation",
        );
        status.active_node = "implementer".to_string();
        status.status = "blocked".to_string();

        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = "run-host-bridge-pending-original-packet".to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_packet_path = None;

        let context = trusted_host_bridge_completion_request_context(
            &root,
            "run-host-bridge-pending-original-packet",
            &request_path.display().to_string(),
            Some(&status),
            &receipt,
        )
        .expect("pending bridge receipt should not require downstream packet evidence")
        .expect("pending bridge request should return completion context");

        assert_eq!(context.dispatch_target, "implementer");
        assert_eq!(
            context.packet_path,
            canonicalize_existing_regular_state_path(&root, &packet_path, "packet")
                .expect("canonical packet")
                .display()
                .to_string()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn retryable_host_bridge_completion_requires_receipt_target() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-receipt-bound-target-{}-{}",
            std::process::id(),
            nanos
        ));
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/forged.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": "run-host-bridge-retryable-receipt-target",
                "dispatch_target": "implementer",
                "downstream_dispatch_active_target": "implementer",
                "downstream_dispatch_status": "blocked"
            })
            .to_string(),
        )
        .expect("write packet");
        let request_path = root.join("host-tool-bridge/requests/retryable-target.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "retryable_blocked",
                "request_id": "retryable-target",
                "run_id": "run-host-bridge-retryable-receipt-target",
                "task_id": "run-host-bridge-retryable-receipt-target",
                "dispatch_target": "implementer",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "dispatch_transport": "host_tool_bridge"
            })
            .to_string(),
        )
        .expect("write request");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-host-bridge-retryable-receipt-target",
            "implementation",
            "implementation",
        );
        status.active_node = "implementer".to_string();
        status.status = "blocked".to_string();

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = "run-host-bridge-retryable-receipt-target".to_string();
        receipt.dispatch_target = "coach".to_string();
        receipt.downstream_dispatch_packet_path = Some(packet_path.display().to_string());

        let error = match trusted_host_bridge_completion_request_context(
            &root,
            "run-host-bridge-retryable-receipt-target",
            &request_path.display().to_string(),
            Some(&status),
            &receipt,
        ) {
            Err(error) => error,
            Ok(_) => panic!("retryable target rebinding must be rejected"),
        };

        assert!(error.contains(
            "Retryable host bridge request dispatch target does not match persisted dispatch receipt evidence"
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn owned_paths_from_lane_packet_uses_active_packet_body_scope_only() {
        let packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "owned_paths": ["secret", 0],
            "delivery_task_packet": {
                "owned_paths": ["allowed"]
            }
        });

        assert_eq!(owned_paths_from_lane_packet(&packet), vec!["allowed"]);

        let malformed_active_scope = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "owned_paths": ["secret"],
            "delivery_task_packet": {
                "owned_paths": ["allowed", 0]
            }
        });

        assert!(owned_paths_from_lane_packet(&malformed_active_scope).is_empty());
    }

    #[tokio::test]
    async fn host_bridge_implementation_scope_uses_immutable_packet() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-immutable-scope-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-host-bridge-immutable-scope";
        let _task = store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge immutable packet scope",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["allowed".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");

        let artifact_path = root.join("attempt-artifacts/immutable-scope.json");
        std::fs::create_dir_all(artifact_path.parent().expect("artifact parent"))
            .expect("create artifact parent");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": ["secret/outside.txt"]
            })
            .to_string(),
        )
        .expect("write out-of-scope artifact");
        store
            .record_task_attempt(crate::state_store::RecordTaskAttemptRequest {
                attempt_id: Some("immutable-scope-attempt".to_string()),
                task_id: run_id.to_string(),
                stage_id: "implementation".to_string(),
                backend: "internal_subagents".to_string(),
                model_profile: "mini".to_string(),
                isolation: "patch_proposal".to_string(),
                freshness: None,
                status: "accepted".to_string(),
                artifact_refs: vec![artifact_path.display().to_string()],
                consolidation_receipt_id: Some("immutable-scope-receipt".to_string()),
                selected_model_profile_readiness_status: None,
                budget_posture: None,
                cap_posture: None,
                write_scope_classification: None,
            })
            .await
            .expect("record task attempt");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("coach".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.implementer".to_string();
        status.recovery_ready = false;
        status.policy_gate = "host_tool_bridge_adapter_required".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_path =
            root.join("runtime-consumption/downstream-dispatch-packets/run-immutable-scope.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": run_id,
                "dispatch_target": "implementer",
                "activation_runtime_role": "worker",
                "packet_template_kind": "delivery_task_packet",
                "owned_paths": ["allowed"],
                "read_only_paths": ["crates/vida/src"],
                "delivery_task_packet": {
                    "goal": "Complete host bridge lane evidence.",
                    "scope_in": ["dispatch_target:implementer"],
                    "handoff_task_class": "implementation",
                    "handoff_runtime_role": "worker",
                    "owned_paths": ["allowed"],
                    "read_only_paths": ["crates/vida/src"],
                    "definition_of_done": ["scope comes from immutable packet"],
                    "verification_command": "cargo test -p vida host_bridge_implementation_scope_uses_immutable_packet",
                    "proof_target": "host bridge completion receipt",
                    "stop_rules": ["stop if bridge evidence is missing"],
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
        .expect("write packet");

        let request_path = root.join("host-tool-bridge/requests/run-immutable-scope.json");
        let result_path = root.join("host-tool-bridge/results/run-immutable-scope.json");
        let bridge_receipt_path = root.join("host-tool-bridge/receipts/run-immutable-scope.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "run-immutable-scope",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "implementer",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "carrier_id": "mini",
                "execution_boundary": "parent_host_session",
                "dispatch_transport": "host_tool_bridge",
                "implementation_isolation": {
                    "schema_version": "implementation-isolation-v1",
                    "artifact_contract": "stage_attempt_implementation_artifact_v1",
                    "owned_paths": ["allowed", "secret"]
                },
                "result_path": result_path.display().to_string(),
                "receipt_path": bridge_receipt_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write request");

        let activation_result_path =
            root.join("runtime-consumption/dispatch-results/run-immutable-scope-activation.json");
        std::fs::create_dir_all(activation_result_path.parent().expect("activation parent"))
            .expect("create activation parent");
        std::fs::write(
            &activation_result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "bridge_request_pending",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "result_path": result_path.display().to_string(),
                    "receipt_path": bridge_receipt_path.display().to_string()
                }
            })
            .to_string(),
        )
        .expect("write activation result");
        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["pending_implementation_evidence".to_string()];
        receipt.downstream_dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_active_target = Some("implementer".to_string());
        receipt.selected_backend = Some("internal_subagents".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        store.close().await;
        wait_for_state_unlock(&root);

        let exit = run_lane(ProxyArgs {
            args: vec![
                "complete".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "host-bridge-immutable-scope".to_string(),
                "--host-bridge-request".to_string(),
                request_path.display().to_string(),
                "--host-agent-id".to_string(),
                "agent-1".to_string(),
                "--host-bridge-summary".to_string(),
                "internal agent completed".to_string(),
                "--json".to_string(),
            ],
        })
        .await;
        assert_eq!(exit, ExitCode::from(2));

        let result: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&result_path).expect("read host bridge result"),
        )
        .expect("host bridge result should be json");
        assert_eq!(result["status"], "blocked");
        assert!(result["blocker_codes"]
            .as_array()
            .expect("blocker codes")
            .iter()
            .any(|code| code == "implementation_attempt_scope_guard_violation"));
        assert_eq!(
            result["scope_validation"]["owned_paths"],
            serde_json::json!(["allowed"])
        );
        assert_eq!(
            result["scope_validation"]["out_of_scope_paths"],
            serde_json::json!(["secret/outside.txt"])
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_complete_host_bridge_rejects_unverified_request_artifacts() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-unverified-request-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-host-bridge-unverified-request";
        let task = store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge unverified request artifact",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/lib.rs".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.implementer".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_path =
            root.join("runtime-consumption/downstream-dispatch-packets/run-unverified.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": run_id,
                "dispatch_target": "implementer",
                "activation_runtime_role": "worker",
                "packet_template_kind": "delivery_task_packet",
                "owned_paths": ["crates/vida/src/lib.rs"],
                "read_only_paths": ["crates/vida/src"],
                "delivery_task_packet": {
                    "goal": "Complete host bridge lane evidence.",
                    "scope_in": ["dispatch_target:implementer"],
                    "handoff_task_class": "implementation",
                    "handoff_runtime_role": "worker",
                    "owned_paths": ["crates/vida/src/lib.rs"],
                    "read_only_paths": ["crates/vida/src"],
                    "definition_of_done": ["request artifact evidence is receipt-backed"],
                    "verification_command": "cargo test -p vida lane_complete_host_bridge_rejects_unverified_request_artifacts",
                    "proof_target": "host bridge completion receipt",
                    "stop_rules": ["stop if bridge evidence is missing"],
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
        .expect("write packet");

        let request_path = root.join("host-tool-bridge/requests/run-unverified.json");
        let result_path = root.join("host-tool-bridge/results/run-unverified.json");
        let bridge_receipt_path = root.join("host-tool-bridge/receipts/run-unverified.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "run-unverified",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "implementer",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "carrier_id": "middle",
                "execution_boundary": "parent_host_session",
                "dispatch_transport": "host_tool_bridge",
                "implementation_isolation": {
                    "schema_version": "implementation-isolation-v1",
                    "artifact_contract": "stage_attempt_implementation_artifact_v1",
                    "owned_paths": ["crates/vida/src/lib.rs"]
                },
                "implementation_artifacts": [{
                    "artifact_kind": "patch_proposal",
                    "attempt_id": "request-attempt-1",
                    "task_id": run_id,
                    "stage_id": "implementation",
                    "freshness": task.updated_at,
                    "receipt_backed": true,
                    "consolidation_receipt_id": "self-attested-request-receipt",
                    "changed_files": ["crates/vida/src/lib.rs"]
                }],
                "result_path": result_path.display().to_string(),
                "receipt_path": bridge_receipt_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write request");
        let activation_result_path =
            root.join("runtime-consumption/dispatch-results/run-unverified-activation.json");
        std::fs::create_dir_all(activation_result_path.parent().expect("activation parent"))
            .expect("create activation parent");
        std::fs::write(
            &activation_result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "bridge_request_pending",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "result_path": result_path.display().to_string(),
                    "receipt_path": bridge_receipt_path.display().to_string()
                }
            })
            .to_string(),
        )
        .expect("write activation result");
        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
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
                "host-bridge-unverified-request".to_string(),
                "--host-bridge-request".to_string(),
                request_path.display().to_string(),
                "--host-agent-id".to_string(),
                "agent-1".to_string(),
                "--host-bridge-summary".to_string(),
                "internal agent completed".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::from(2));

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store");
        let blocked = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read blocked receipt")
            .expect("receipt should exist");
        assert_eq!(
            blocked.blocker_code.as_deref(),
            Some("implementation_artifact_receipt_unverified")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_complete_host_bridge_attempt_scope_block_can_retry_without_consuming_attempt() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-attempt-retry-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-host-bridge-attempt-retry";
        let task = store
            .create_task_with_fixture_parent(crate::state_store::CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge attempt artifact retry",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/lib.rs".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");

        let artifact_path = root.join("attempt-artifacts/attempt-retry.json");
        std::fs::create_dir_all(artifact_path.parent().expect("artifact parent"))
            .expect("create artifact parent");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": ["crates/vida/src/root_command_router.rs"]
            })
            .to_string(),
        )
        .expect("write out-of-scope artifact");
        store
            .record_task_attempt(crate::state_store::RecordTaskAttemptRequest {
                attempt_id: Some("attempt-retry".to_string()),
                task_id: run_id.to_string(),
                stage_id: "implementation".to_string(),
                backend: "internal_subagents".to_string(),
                model_profile: "middle".to_string(),
                isolation: "patch_proposal".to_string(),
                freshness: None,
                status: "accepted".to_string(),
                artifact_refs: vec![artifact_path.display().to_string()],
                consolidation_receipt_id: Some("attempt-retry-consolidation-receipt".to_string()),
                selected_model_profile_readiness_status: None,
                budget_posture: None,
                cap_posture: None,
                write_scope_classification: None,
            })
            .await
            .expect("record accepted attempt");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementer_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.implementer".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_path =
            root.join("runtime-consumption/downstream-dispatch-packets/run-attempt-retry.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": run_id,
                "dispatch_target": "implementer",
                "activation_runtime_role": "worker",
                "packet_template_kind": "delivery_task_packet",
                "owned_paths": ["crates/vida/src/lib.rs"],
                "read_only_paths": ["crates/vida/src"],
                "delivery_task_packet": {
                    "goal": "Complete host bridge lane evidence.",
                    "scope_in": ["dispatch_target:implementer"],
                    "handoff_task_class": "implementation",
                    "handoff_runtime_role": "worker",
                    "owned_paths": ["crates/vida/src/lib.rs"],
                    "read_only_paths": ["crates/vida/src"],
                    "definition_of_done": ["attempt artifact evidence is accepted"],
                    "verification_command": "cargo test -p vida lane_complete_host_bridge_attempt_scope_block_can_retry_without_consuming_attempt",
                    "proof_target": "host bridge completion receipt",
                    "stop_rules": ["stop if bridge evidence is missing"],
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
        .expect("write packet");

        let request_path = root.join("host-tool-bridge/requests/run-attempt-retry.json");
        let result_path = root.join("host-tool-bridge/results/run-attempt-retry.json");
        let bridge_receipt_path = root.join("host-tool-bridge/receipts/run-attempt-retry.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "run-attempt-retry",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "implementer",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "carrier_id": "middle",
                "execution_boundary": "parent_host_session",
                "dispatch_transport": "host_tool_bridge",
                "implementation_isolation": {
                    "schema_version": "implementation-isolation-v1",
                    "artifact_contract": "stage_attempt_implementation_artifact_v1",
                    "owned_paths": ["crates/vida/src/lib.rs"]
                },
                "implementation_artifacts": [{
                    "artifact_kind": "patch_proposal",
                    "attempt_id": "attempt-retry",
                    "task_id": run_id,
                    "stage_id": "implementation",
                    "freshness": task.updated_at,
                    "receipt_backed": true,
                    "consolidation_receipt_id": "attempt-retry-consolidation-receipt",
                    "changed_files": ["crates/vida/src/lib.rs"]
                }],
                "result_path": result_path.display().to_string(),
                "receipt_path": bridge_receipt_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write request");
        let activation_result_path =
            root.join("runtime-consumption/dispatch-results/run-attempt-retry-activation.json");
        std::fs::create_dir_all(activation_result_path.parent().expect("activation parent"))
            .expect("create activation parent");
        std::fs::write(
            &activation_result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "bridge_request_pending",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "result_path": result_path.display().to_string(),
                    "receipt_path": bridge_receipt_path.display().to_string()
                }
            })
            .to_string(),
        )
        .expect("write activation result");
        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
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

        let blocked_args = ProxyArgs {
            args: vec![
                "complete".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "host-bridge-attempt-scope-blocked".to_string(),
                "--host-bridge-request".to_string(),
                request_path.display().to_string(),
                "--host-agent-id".to_string(),
                "agent-1".to_string(),
                "--host-bridge-summary".to_string(),
                "internal agent completed".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(blocked_args).await, ExitCode::from(2));

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after blocked completion");
        let blocked = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read blocked receipt")
            .expect("receipt should exist");
        assert_eq!(
            blocked.blocker_code.as_deref(),
            Some("implementation_attempt_scope_guard_violation")
        );
        let attempt_after_block = store
            .task_attempt("attempt-retry")
            .await
            .expect("attempt should remain readable");
        assert_eq!(attempt_after_block.status, "accepted");
        drop(store);
        wait_for_state_unlock(&root);

        let missing_request_retry_args = ProxyArgs {
            args: vec![
                "complete".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "host-bridge-attempt-scope-no-request".to_string(),
                "--host-agent-id".to_string(),
                "agent-1".to_string(),
                "--host-bridge-summary".to_string(),
                "internal agent completed".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(
            run_lane(missing_request_retry_args).await,
            ExitCode::from(2)
        );
        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after missing request retry");
        let still_blocked = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read still blocked receipt")
            .expect("receipt should exist");
        assert_eq!(still_blocked.dispatch_status, "blocked");
        assert_eq!(
            still_blocked.blocker_code.as_deref(),
            Some("implementation_attempt_scope_guard_violation")
        );
        assert_eq!(still_blocked.downstream_dispatch_ready, false);
        assert_eq!(
            still_blocked.downstream_dispatch_status.as_deref(),
            Some("blocked")
        );
        drop(store);
        wait_for_state_unlock(&root);

        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": ["crates/vida/src/lib.rs"]
            })
            .to_string(),
        )
        .expect("write corrected artifact");
        let retry_args = ProxyArgs {
            args: vec![
                "complete".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "host-bridge-attempt-scope-retry".to_string(),
                "--host-bridge-request".to_string(),
                request_path.display().to_string(),
                "--host-agent-id".to_string(),
                "agent-1".to_string(),
                "--host-bridge-summary".to_string(),
                "internal agent completed".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(retry_args).await, ExitCode::SUCCESS);
        let bridge_result: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&result_path).expect("read retried host bridge result"),
        )
        .expect("host bridge result should be json");
        assert_eq!(bridge_result["status"], "pass");
        assert_eq!(bridge_result["scope_validation"]["status"], "pass");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_complete_host_bridge_blocker_does_not_complete_verification_or_open_closure() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-blocker-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-host-bridge-verifier-blocker";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "verification".to_string();
        status.next_node = Some("closure".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "verification_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.verification".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_path = root.join(
            "runtime-consumption/downstream-dispatch-packets/run-host-bridge-verification.json",
        );
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
                "dispatch_target": "verification",
                "activation_runtime_role": "verifier",
                "packet_template_kind": "delivery_task_packet",
                "owned_paths": ["crates/vida/src/lane_surface.rs"],
                "read_only_paths": ["crates/vida/src"],
                "delivery_task_packet": {
                    "goal": "Verify implementation evidence.",
                    "scope_in": ["dispatch_target:verification"],
                    "handoff_task_class": "implementation",
                    "handoff_runtime_role": "verifier",
                    "owned_paths": ["crates/vida/src/lane_surface.rs"],
                    "read_only_paths": ["crates/vida/src"],
                    "definition_of_done": ["verification either approves closure or blocks rework"],
                    "verification_command": "cargo test -p vida lane_complete",
                    "proof_target": "verification receipt",
                    "stop_rules": ["stop if implementation evidence is missing"],
                    "blocking_question": "none"
                },
                "downstream_dispatch_target": "closure",
                "downstream_dispatch_active_target": "verification",
                "downstream_dispatch_ready": false,
                "downstream_dispatch_blockers": ["pending_verification_evidence"],
                "downstream_dispatch_status": "blocked",
                "downstream_lane_status": "lane_blocked"
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let request_path = root.join("host-tool-bridge/requests/run-host-bridge-verification.json");
        let result_path = root.join("host-tool-bridge/results/run-host-bridge-verification.json");
        let bridge_receipt_path =
            root.join("host-tool-bridge/receipts/run-host-bridge-verification.json");
        std::fs::create_dir_all(
            request_path
                .parent()
                .expect("request path should have parent"),
        )
        .expect("create request dir");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "run-host-bridge-verification",
                "run_id": run_id,
                "dispatch_target": "verification",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "carrier_id": "verifier",
                "execution_boundary": "parent_host_session",
                "dispatch_transport": "host_tool_bridge",
                "result_path": result_path.display().to_string(),
                "receipt_path": bridge_receipt_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write host bridge request");

        let activation_result_path = root.join(
            "runtime-consumption/dispatch-results/run-host-bridge-verification-activation.json",
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
                "status": "blocked",
                "execution_state": "bridge_request_pending",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "result_path": result_path.display().to_string(),
                    "receipt_path": bridge_receipt_path.display().to_string()
                }
            })
            .to_string(),
        )
        .expect("write activation result");

        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "verification".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());
        receipt.downstream_dispatch_target = Some("closure".to_string());
        receipt.downstream_dispatch_command = Some("vida taskflow dispatch closure".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["pending_verification_evidence".to_string()];
        receipt.downstream_dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_active_target = Some("verification".to_string());
        receipt.selected_backend = Some("internal_subagents".to_string());
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
                "host-bridge-verification-blocker".to_string(),
                "--host-bridge-request".to_string(),
                request_path.display().to_string(),
                "--host-agent-id".to_string(),
                "verifier-1".to_string(),
                "--host-bridge-summary".to_string(),
                "verdict: blocker; rework required; product implementation evidence missing; not closure-ready".to_string(),
                "--json".to_string(),
            ],
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
        assert_eq!(after.dispatch_status, "blocked");
        assert_eq!(after.lane_status, crate::LaneStatus::LaneBlocked.as_str());
        assert!(!after.downstream_dispatch_ready);
        assert_eq!(
            after.blocker_code.as_deref(),
            Some("verification_rework_required")
        );
        assert_eq!(
            after.downstream_dispatch_blockers,
            vec!["verification_rework_required".to_string()]
        );

        let current_status = store
            .run_graph_status(run_id)
            .await
            .expect("read run graph status");
        assert_eq!(current_status.active_node, "verification");
        assert_eq!(current_status.lifecycle_stage, "verification_blocked");

        let bridge_result: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&result_path).expect("read host bridge result"),
        )
        .expect("host bridge result should be json");
        assert_eq!(bridge_result["status"], "blocked");
        assert_eq!(bridge_result["execution_state"], "blocked");
        assert_eq!(
            bridge_result["blocker_code"],
            "verification_rework_required"
        );
        assert_eq!(bridge_result["execution_evidence"]["receipt_backed"], true);

        let completion_result_path = after
            .downstream_dispatch_result_path
            .clone()
            .expect("completion result path should be recorded");
        let completion_result: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(completion_result_path).expect("read lane completion result"),
        )
        .expect("lane completion result should be json");
        assert_eq!(completion_result["status"], "blocked");
        assert_eq!(
            completion_result["blocker_code"],
            "verification_rework_required"
        );
        assert_eq!(completion_result["closure_ready"], false);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_no_request_redirect() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-host-bridge-no-request-redirect-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-host-bridge-no-request-redirect";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "verification".to_string();
        status.next_node = Some("closure".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "verification_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.verification".to_string();
        status.recovery_ready = false;
        status.policy_gate = "host_tool_bridge_adapter_required".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_path = root.join(
            "runtime-consumption/downstream-dispatch-packets/run-host-bridge-verification.json",
        );
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": run_id,
                "dispatch_target": "verification",
                "activation_runtime_role": "verifier",
                "packet_template_kind": "delivery_task_packet",
                "owned_paths": ["crates/vida/src/lane_surface.rs"],
                "read_only_paths": ["crates/vida/src"],
                "delivery_task_packet": {
                    "goal": "Verify implementation evidence.",
                    "scope_in": ["dispatch_target:verification"],
                    "handoff_task_class": "implementation",
                    "handoff_runtime_role": "verifier",
                    "owned_paths": ["crates/vida/src/lane_surface.rs"],
                    "read_only_paths": ["crates/vida/src"],
                    "definition_of_done": ["verification either approves closure or blocks rework"],
                    "verification_command": "cargo test -p vida lane_complete",
                    "proof_target": "verification receipt",
                    "stop_rules": ["stop if implementation evidence is missing"],
                    "blocking_question": "none"
                },
                "downstream_dispatch_target": "closure",
                "downstream_dispatch_active_target": "verification",
                "downstream_dispatch_ready": false,
                "downstream_dispatch_blockers": ["pending_verification_evidence"],
                "downstream_dispatch_status": "blocked",
                "downstream_lane_status": "lane_blocked"
            })
            .to_string(),
        )
        .expect("write packet");

        let request_path = root.join("host-tool-bridge/requests/run-host-bridge-verification.json");
        let authoritative_result_path =
            root.join("host-tool-bridge/results/run-host-bridge-authoritative.json");
        let authoritative_receipt_path =
            root.join("host-tool-bridge/receipts/run-host-bridge-authoritative.json");
        let redirected_result_path =
            root.join("host-tool-bridge/results/run-host-bridge-redirected.json");
        let redirected_receipt_path =
            root.join("host-tool-bridge/receipts/run-host-bridge-redirected.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request dir");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "run-host-bridge-no-request-redirect",
                "run_id": run_id,
                "dispatch_target": "verification",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "carrier_id": "verifier",
                "execution_boundary": "parent_host_session",
                "dispatch_transport": "host_tool_bridge",
                "result_path": redirected_result_path.display().to_string(),
                "receipt_path": redirected_receipt_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write host bridge request");

        let activation_result_path = root.join(
            "runtime-consumption/dispatch-results/run-host-bridge-no-request-redirect-activation.json",
        );
        std::fs::create_dir_all(activation_result_path.parent().expect("activation parent"))
            .expect("create activation parent");
        std::fs::write(
            &activation_result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "bridge_request_pending",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "result_path": authoritative_result_path.display().to_string(),
                    "receipt_path": authoritative_receipt_path.display().to_string()
                }
            })
            .to_string(),
        )
        .expect("write activation result");

        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "verification".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());
        receipt.downstream_dispatch_target = Some("closure".to_string());
        receipt.downstream_dispatch_command = Some("vida taskflow dispatch closure".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["pending_verification_evidence".to_string()];
        receipt.downstream_dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_active_target = Some("verification".to_string());
        receipt.selected_backend = Some("internal_subagents".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        let persisted_receipt = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read persisted receipt")
            .expect("persisted receipt should exist");
        let activation_result_path_string = activation_result_path.display().to_string();
        store.close().await;
        wait_for_state_unlock(&root);

        let error = materialize_host_bridge_completion_evidence(
            &root,
            request_path.to_str().expect("utf8 request path"),
            run_id,
            "verification",
            &persisted_receipt,
            "host-bridge-no-request-redirect",
            Some("verifier-1"),
            Some("internal agent completed"),
            HostBridgeTaskflowImplementationEvidence::default(),
            &[],
            false,
            false,
        )
        .expect_err("redirected request should fail closed");
        assert!(
            error.contains("do not match persisted dispatch receipt evidence"),
            "unexpected error: {error}"
        );

        let exit = run_lane(ProxyArgs {
            args: vec![
                "complete".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "host-bridge-no-request-redirect".to_string(),
                "--host-bridge-request".to_string(),
                request_path.display().to_string(),
                "--host-agent-id".to_string(),
                "verifier-1".to_string(),
                "--host-bridge-summary".to_string(),
                "internal agent completed".to_string(),
                "--state-dir".to_string(),
                root.display().to_string(),
                "--json".to_string(),
            ],
        })
        .await;
        assert_eq!(exit, ExitCode::from(2));

        assert!(!redirected_result_path.exists());
        assert!(!redirected_receipt_path.exists());
        assert!(!authoritative_result_path.exists());
        assert!(!authoritative_receipt_path.exists());

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt after")
            .expect("receipt should exist");
        assert_eq!(
            after.dispatch_result_path.as_deref(),
            Some(activation_result_path_string.as_str())
        );
        assert_eq!(after.dispatch_status, "bridge_request_pending");
        store.close().await;

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_completion_accepts_configured_state_root_subdirectories() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-configured-complete-{}-{nanos}",
            std::process::id()
        ));
        let request_path = root.join("custom-agent-bridge/requests/run-custom.json");
        let result_path = root.join("custom-agent-bridge/results/run-custom.json");
        let receipt_path = root.join("custom-agent-bridge/receipts/run-custom.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "run-custom",
                "run_id": "run-custom",
                "dispatch_target": "implementer",
                "packet_path": root.join("runtime-consumption/downstream-dispatch-packets/run-custom.json").display().to_string(),
                "backend_id": "internal_subagents",
                "dispatch_transport": "host_tool_bridge",
                "result_path": result_path.display().to_string(),
                "receipt_path": receipt_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write request");
        let activation_result_path =
            root.join("runtime-consumption/dispatch-results/run-custom-activation.json");
        std::fs::create_dir_all(activation_result_path.parent().expect("activation parent"))
            .expect("create activation parent");
        std::fs::write(
            &activation_result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "bridge_request_pending",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "result_path": result_path.display().to_string(),
                    "receipt_path": receipt_path.display().to_string()
                }
            })
            .to_string(),
        )
        .expect("write activation result");
        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = "run-custom".to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());

        let evidence = materialize_host_bridge_completion_evidence(
            &root,
            request_path.to_str().expect("utf8 request path"),
            "run-custom",
            "implementer",
            &receipt,
            "receipt-custom",
            None,
            None,
            HostBridgeTaskflowImplementationEvidence::default(),
            &[],
            false,
            false,
        )
        .expect("configured in-state bridge paths should be accepted");

        assert_eq!(evidence.result_path, result_path.display().to_string());
        assert_eq!(evidence.receipt_path, receipt_path.display().to_string());
        assert!(result_path.exists());
        assert!(receipt_path.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_completion_rejects_artifact_paths_outside_state_root() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-path-guard-{}-{nanos}",
            std::process::id()
        ));
        let outside_result_path = std::env::temp_dir().join(format!(
            "vida-host-bridge-outside-result-{}-{nanos}.json",
            std::process::id()
        ));
        let request_path = root.join("host-tool-bridge/requests/run-guard.json");
        let receipt_path = root.join("host-tool-bridge/receipts/run-guard.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "run-guard",
                "run_id": "run-guard",
                "dispatch_target": "implementer",
                "packet_path": root.join("runtime-consumption/downstream-dispatch-packets/run-guard.json").display().to_string(),
                "backend_id": "internal_subagents",
                "dispatch_transport": "host_tool_bridge",
                "result_path": outside_result_path.display().to_string(),
                "receipt_path": receipt_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write request");
        let activation_result_path =
            root.join("runtime-consumption/dispatch-results/run-guard-activation.json");
        std::fs::create_dir_all(activation_result_path.parent().expect("activation parent"))
            .expect("create activation parent");
        std::fs::write(
            &activation_result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "bridge_request_pending",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "result_path": outside_result_path.display().to_string(),
                    "receipt_path": receipt_path.display().to_string()
                }
            })
            .to_string(),
        )
        .expect("write activation result");
        let mut receipt = sample_receipt("bridge_request_pending");
        receipt.run_id = "run-guard".to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());

        let error = materialize_host_bridge_completion_evidence(
            &root,
            request_path.to_str().expect("utf8 request path"),
            "run-guard",
            "implementer",
            &receipt,
            "receipt-guard",
            None,
            None,
            HostBridgeTaskflowImplementationEvidence::default(),
            &[],
            false,
            false,
        )
        .expect_err("outside result path should be rejected");

        assert!(
            error.contains("outside VIDA state root"),
            "unexpected error: {error}"
        );
        assert!(!outside_result_path.exists());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&outside_result_path);
    }

    #[tokio::test]
    async fn lane_complete_preserves_source_context_for_duplicate_lane_targets() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-duplicate-complete-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-complete-duplicate";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "coach".to_string();
        status.next_node = Some("coach".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_coach".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.coach_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut role_selection = lane_complete_role_selection(run_id);
        role_selection.execution_plan["development_flow"]["dispatch_contract"]
            ["execution_lane_sequence"] = serde_json::json!([
            "test_author",
            "coach",
            "implementer",
            "coach",
            "verification"
        ]);
        let packet_path = root.join(
            "runtime-consumption/downstream-dispatch-packets/run-lane-complete-duplicate.json",
        );
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
                "source_dispatch_target": "implementer",
                "dispatch_target": "coach",
                "activation_runtime_role": "coach",
                "packet_template_kind": "delivery_task_packet",
                "owned_paths": ["crates/vida/src/lane_surface.rs"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "delivery_task_packet": {
                    "goal": "Complete the second coach lane evidence.",
                    "scope_in": ["dispatch_target:coach"],
                    "handoff_task_class": "review",
                    "handoff_runtime_role": "coach",
                    "owned_paths": ["crates/vida/src/lane_surface.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption"],
                    "definition_of_done": ["second coach completion advances to verification"],
                    "verification_command": "cargo test -p vida lane_complete_preserves_source_context_for_duplicate_lane_targets",
                    "proof_target": "duplicate coach lane advances to verification",
                    "stop_rules": ["stop if packet contract is invalid"],
                    "blocking_question": "none"
                },
                "role_selection_full": role_selection,
                "run_graph_bootstrap": {
                    "run_id": run_id
                },
                "downstream_dispatch_target": "verification",
                "downstream_dispatch_active_target": "coach",
                "downstream_dispatch_ready": false,
                "downstream_dispatch_blockers": ["pending_review_clean_evidence"],
                "downstream_dispatch_status": "blocked",
                "downstream_lane_status": "lane_blocked"
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "coach".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_command = Some("vida agent-init".to_string());
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_target = Some("verification".to_string());
        receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
        receipt.downstream_dispatch_note =
            Some("after `coach` evidence is recorded, activate `verification`".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["pending_review_clean_evidence".to_string()];
        receipt.downstream_dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_active_target = Some("coach".to_string());
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
                "completion-duplicate-1".to_string(),
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
        assert_eq!(
            after.downstream_dispatch_target.as_deref(),
            Some("verification")
        );
        assert!(
            after
                .downstream_dispatch_packet_path
                .as_deref()
                .is_some_and(|value| value.contains("downstream-dispatch-packets")),
            "lane completion should materialize a verification packet"
        );

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
            run_id: Some(run_id.to_string()),
            dispatch_target: Some("implementer".to_string()),
            dispatch_packet_path: receipt.dispatch_packet_path.clone(),
            source_exception_path_receipt_id: Some("exception-1".to_string()),
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
        assert_eq!(advanced_status.lifecycle_stage, "implementer_complete");
        assert_eq!(advanced_status.handoff_state, "awaiting_coach");
        assert_eq!(advanced_status.resume_target, "dispatch.coach_lane");
        assert!(advanced_status.recovery_ready);
        assert_eq!(binding.binding_source, "lane_complete");
        assert_eq!(binding.active_bounded_unit["kind"], "run_graph_task");
        assert_eq!(binding.active_bounded_unit["active_node"], "implementer");

        let _ = std::fs::remove_dir_all(&root);
    }
}
