#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskflowDiagnosticKind {
    InternalActivationViewOnly,
    StaleMissingTaskRunGraph,
    DispatchPacketContractInvalid,
    ContinuationBindingAmbiguous,
    ContinuationBindingMismatch,
    ContinuationBindingNotResumeable,
    RunGraphRecoveryNotReady,
    MissingRunGraphDispatchReceipt,
    RuntimeDispatchHandoffTimeout,
    ConsumeContinueResumeBlocked,
}

impl TaskflowDiagnosticKind {
    pub(crate) fn blocker_code(self) -> &'static str {
        match self {
            Self::InternalActivationViewOnly => "internal_activation_view_only",
            Self::StaleMissingTaskRunGraph => "stale_missing_task_run_graph",
            Self::DispatchPacketContractInvalid => "dispatch_packet_contract_invalid",
            Self::ContinuationBindingAmbiguous => "continuation_binding_ambiguous",
            Self::ContinuationBindingMismatch => "continuation_binding_mismatch",
            Self::ContinuationBindingNotResumeable => "continuation_binding_not_resumeable",
            Self::RunGraphRecoveryNotReady => "run_graph_recovery_not_ready",
            Self::MissingRunGraphDispatchReceipt => "missing_run_graph_dispatch_receipt",
            Self::RuntimeDispatchHandoffTimeout => "runtime_dispatch_handoff_timeout",
            Self::ConsumeContinueResumeBlocked => "consume_continue_resume_blocked",
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct DispatchPacketRepairRefs {
    pub(crate) run_id: Option<String>,
    pub(crate) task_id: Option<String>,
}

#[derive(Debug)]
pub(crate) struct TaskflowDiagnosticDecision {
    pub(crate) kind: TaskflowDiagnosticKind,
    pub(crate) run_id: Option<String>,
    pub(crate) task_id: Option<String>,
    pub(crate) dispatch_packet_path: Option<String>,
    pub(crate) next_actions: Vec<String>,
    pub(crate) artifact_refs: serde_json::Value,
}

pub(crate) fn diagnose_consume_resume_error(error: &str) -> TaskflowDiagnosticDecision {
    let kind = consume_resume_error_kind(error);
    let packet_path = consume_resume_error_packet_path(error);
    let packet_refs = packet_path
        .as_deref()
        .map(dispatch_packet_repair_refs_from_path)
        .unwrap_or_default();
    let run_id = consume_resume_error_run_id(error).or(packet_refs.run_id);
    let task_id = packet_refs.task_id;
    let next_actions =
        consume_resume_next_actions(kind, error, run_id.as_deref(), task_id.as_deref());
    let artifact_refs = serde_json::json!({
        "run_id": run_id,
        "task_id": task_id,
        "dispatch_packet_path": packet_path,
    });
    TaskflowDiagnosticDecision {
        kind,
        run_id,
        task_id,
        dispatch_packet_path: packet_path,
        next_actions,
        artifact_refs,
    }
}

pub(crate) fn consume_resume_error_payload(error: &str, surface_name: &str) -> serde_json::Value {
    let decision = diagnose_consume_resume_error(error);
    let blocker_code = decision.kind.blocker_code().to_string();
    let mut artifact_refs = decision.artifact_refs.clone();
    if let Some(object) = artifact_refs.as_object_mut() {
        object.insert("surface".to_string(), serde_json::json!(surface_name));
    }
    let mut extra_fields = serde_json::json!({
        "error": error,
        "run_id": decision.run_id,
    });
    if let Some(object) = extra_fields.as_object_mut() {
        object.insert(
            "diagnostic_kind".to_string(),
            serde_json::json!(decision.kind.blocker_code()),
        );
    }
    crate::release1_operator_output::build_release1_operator_output_payload(
        surface_name,
        vec![blocker_code],
        decision.next_actions,
        artifact_refs,
        extra_fields,
    )
    .expect("consume resume diagnostics should build release-1 operator payload")
}

pub(crate) fn consume_resume_error_blocker_code(error: &str) -> &'static str {
    consume_resume_error_kind(error).blocker_code()
}

fn consume_resume_error_kind(error: &str) -> TaskflowDiagnosticKind {
    if error.contains("materialization-only dispatch receipt")
        || error.contains("task-materialization result")
    {
        TaskflowDiagnosticKind::InternalActivationViewOnly
    } else if error.contains("Stale missing-task run graph")
        || error.contains("references missing TaskFlow task")
    {
        TaskflowDiagnosticKind::StaleMissingTaskRunGraph
    } else if error.contains("missing required packet fields")
        || error.contains("dispatch packet contract invalid")
        || error.contains("dispatch_packet_contract_invalid")
        || error.contains("missing_owned_write_scope")
        || error.contains("missing_owned_paths")
    {
        TaskflowDiagnosticKind::DispatchPacketContractInvalid
    } else if error.contains("Latest continuation binding") && error.contains("ambiguous") {
        TaskflowDiagnosticKind::ContinuationBindingAmbiguous
    } else if error.contains("Latest explicit continuation binding points to run") {
        TaskflowDiagnosticKind::ContinuationBindingMismatch
    } else if error.contains("not resumeable through default") {
        TaskflowDiagnosticKind::ContinuationBindingNotResumeable
    } else if error.contains("Run-graph resume gate denied") && error.contains("recovery_ready") {
        TaskflowDiagnosticKind::RunGraphRecoveryNotReady
    } else if error.contains("Persisted dispatch receipt expects dispatch_packet_path") {
        TaskflowDiagnosticKind::ConsumeContinueResumeBlocked
    } else if error.contains("No persisted run-graph dispatch receipt exists")
        || error.contains("missing receipt recovery could not load dispatch context")
    {
        TaskflowDiagnosticKind::MissingRunGraphDispatchReceipt
    } else if error.contains("Timed out executing runtime dispatch handoff") {
        TaskflowDiagnosticKind::RuntimeDispatchHandoffTimeout
    } else {
        TaskflowDiagnosticKind::ConsumeContinueResumeBlocked
    }
}

fn consume_resume_error_run_id(error: &str) -> Option<String> {
    for marker in [
        "Stale missing-task run graph `",
        "Run-graph resume gate denied for `",
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

fn consume_resume_error_packet_path(error: &str) -> Option<String> {
    for marker in [
        "; dispatch packet `",
        "dispatch packet `",
        "packet path `",
        "packet_path `",
    ] {
        let Some(start) = error.find(marker).map(|index| index + marker.len()) else {
            continue;
        };
        let rest = &error[start..];
        let Some(end) = rest.find('`') else {
            continue;
        };
        let path = rest[..end].trim();
        if !path.is_empty() && path != "delivery_task_packet" {
            return Some(path.to_string());
        }
    }
    None
}

pub(crate) fn dispatch_packet_repair_refs_from_path(path: &str) -> DispatchPacketRepairRefs {
    let Ok(project_root) = std::env::current_dir() else {
        return DispatchPacketRepairRefs::default();
    };
    let Some(packet) =
        crate::status_surface::dispatch_packet_json_from_project_path(&project_root, path)
    else {
        return DispatchPacketRepairRefs::default();
    };
    DispatchPacketRepairRefs {
        run_id: dispatch_packet_string_field(&packet, &["run_id"])
            .or_else(|| dispatch_packet_string_field(&packet, &["run_graph_bootstrap", "run_id"])),
        task_id: dispatch_packet_string_field(&packet, &["task_id"])
            .or_else(|| dispatch_packet_string_field(&packet, &["delivery_task_packet", "task_id"]))
            .or_else(|| dispatch_packet_string_field(&packet, &["delivery_task_packet", "id"]))
            .or_else(|| {
                dispatch_packet_string_field(&packet, &["delivery_task_packet", "backlog_id"])
            })
            .or_else(|| dispatch_packet_string_field(&packet, &["run_graph_bootstrap", "task_id"])),
    }
}

fn dispatch_packet_string_field(packet: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut cursor = packet;
    for key in path {
        cursor = cursor.get(*key)?;
    }
    cursor
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn consume_resume_next_actions(
    kind: TaskflowDiagnosticKind,
    error: &str,
    run_id: Option<&str>,
    task_id: Option<&str>,
) -> Vec<String> {
    match kind {
        TaskflowDiagnosticKind::StaleMissingTaskRunGraph => {
            let retire_action = run_id.map_or_else(
                || {
                    "Inspect `vida taskflow recovery latest`; if it reports a missing-task stale run, retire that concrete run with `vida lane retire <run-id> --receipt-id <receipt-id> --reason \"missing TaskFlow task stale run\"`."
                        .to_string()
                },
                |run_id| {
                    format!(
                        "Retire the stale missing-task run with `vida lane retire {} --receipt-id {} --reason \"missing TaskFlow task stale run\"`.",
                        crate::shell_quote(run_id),
                        crate::shell_quote(run_id)
                    )
                },
            );
            vec![
                retire_action,
                "Refresh continuation evidence with `vida status` and `vida taskflow recovery latest` before retrying `vida taskflow consume continue`.".to_string(),
                "Do not bind recovery to the missing TaskFlow task.".to_string(),
            ]
        }
        TaskflowDiagnosticKind::DispatchPacketContractInvalid => {
            let repair_action = match (run_id, task_id) {
                (Some(run_id), Some(task_id)) => {
                    format!(
                        "Repair the persisted dispatch packet from canonical task metadata with `vida taskflow packet repair --run-id {} --from-task {}`, or regenerate the packet from the active bounded task before retrying.",
                        crate::shell_quote(run_id),
                        crate::shell_quote(task_id),
                    )
                }
                (Some(run_id), None) => {
                    let inspect_command = operator_output::command_text::human_command(&format!(
                        "vida taskflow run-graph status {}",
                        crate::shell_quote(run_id)
                    ));
                    format!(
                        "Inspect the dispatch packet owner with `{inspect_command}` before repair; do not run packet repair until a canonical TaskFlow task_id for run `{run_id}` is proven."
                    )
                }
                _ => {
                    "Repair the persisted dispatch packet from canonical task metadata with `vida taskflow packet repair --run-id <run-id> --from-task <task-id>`, or regenerate the packet from the active bounded task before retrying."
                        .to_string()
                }
            };
            vec![
                repair_action,
                "Ensure delivery_task_packet.owned_paths and proof targets are present before running `vida taskflow consume continue`.".to_string(),
                "Do not treat packet-contract failure as a continuation-binding ambiguity.".to_string(),
            ]
        }
        TaskflowDiagnosticKind::ContinuationBindingAmbiguous
            if error.contains("has not reached closure_complete") =>
        {
            let refresh_action = run_id.map_or_else(
                || {
                    "Pass `--run-id <run-id>` only when intentionally refreshing that specific active run."
                        .to_string()
                },
                |run_id| {
                    format!(
                        "Refresh the active run explicitly with `vida taskflow consume continue --run-id {}`.",
                        crate::shell_quote(run_id)
                    )
                },
            );
            vec![
                refresh_action,
                "Do not bind a new --task-id until the source run reaches closure_complete with no downstream target.".to_string(),
                "Inspect run-graph and task evidence if the explicit refresh remains blocked.".to_string(),
            ]
        }
        TaskflowDiagnosticKind::ContinuationBindingAmbiguous => {
            vec![
                "Bind the next bounded unit explicitly with `vida taskflow continuation bind <run-id> --task-id <task-id>`.".to_string(),
                "Pass `--run-id <run-id>` only when intentionally refreshing that specific run.".to_string(),
            ]
        }
        TaskflowDiagnosticKind::RunGraphRecoveryNotReady => {
            vec![
                "Inspect recovery state with `vida taskflow recovery latest`.".to_string(),
                "Do not infer a closure handoff unless recovery exposes a closure packet or closure result.".to_string(),
            ]
        }
        TaskflowDiagnosticKind::InternalActivationViewOnly => {
            vec![
                "Inspect the blocked materialization lane with `vida lane show` for the same run.".to_string(),
                "Do not resume the run until receipt, lane, run-graph, and task identity agree on an executable next packet.".to_string(),
            ]
        }
        _ => {
            vec![
                "Inspect continuation evidence with `vida status` and `vida taskflow recovery latest`.".to_string(),
                "Bind or refresh the intended bounded unit before retrying `vida taskflow consume continue`.".to_string(),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dispatch_packet_test_root(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::current_dir()
            .expect("current dir")
            .join("target")
            .join(format!("{name}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn consume_resume_error_payload_uses_release1_operator_contract_builder() {
        let payload = consume_resume_error_payload(
            "Run-graph resume gate denied for `run-1`: recovery_ready is false",
            "vida taskflow consume continue",
        );

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["run_graph_recovery_not_ready"])
        );
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(
            payload["shared_fields"]["blocker_codes"],
            payload["operator_contracts"]["blocker_codes"]
        );
        assert_eq!(
            payload["shared_fields"]["next_actions"],
            payload["operator_contracts"]["next_actions"]
        );
        assert_eq!(
            payload["operator_contracts"]["contract_id"],
            "release-1-operator-contracts"
        );
        assert_eq!(
            payload["artifact_refs"]["surface"],
            "vida taskflow consume continue"
        );
    }

    #[test]
    fn consume_resume_error_payload_builds_stale_run_retire_action() {
        let payload = consume_resume_error_payload(
            "Stale missing-task run graph `run-stale` references missing TaskFlow task",
            "vida taskflow consume continue",
        );

        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["stale_missing_task_run_graph"])
        );
        assert_eq!(payload["artifact_refs"]["run_id"], "run-stale");
        assert!(payload["next_actions"][0]
            .as_str()
            .expect("next action should be text")
            .contains("vida lane retire run-stale --receipt-id run-stale"));
    }

    #[test]
    fn persisted_dispatch_packet_path_mismatch_is_consume_continue_resume_blocked() {
        let payload = consume_resume_error_payload(
            "Persisted dispatch receipt expects dispatch_packet_path `/state/runtime-consumption/dispatch-packets/run-1.json` but resolved `/state/runtime-consumption/downstream-dispatch-packets/run-1-explicit-ready-downstream.json`",
            "vida taskflow consume continue",
        );

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["consume_continue_resume_blocked"])
        );
        assert_eq!(
            payload["diagnostic_kind"],
            "consume_continue_resume_blocked"
        );
        assert!(payload["error"]
            .as_str()
            .is_some_and(|error| error.contains("expects dispatch_packet_path")));
    }

    #[test]
    fn consume_resume_error_payload_builds_packet_repair_action_from_packet_refs() {
        let dir = unique_dispatch_packet_test_root("vida-taskflow-diagnostics-packet");
        let packet_path = dir.join("packet.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("packet dir should create");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": "run-packet",
                "delivery_task_packet": {
                    "task_id": "task-packet"
                }
            })
            .to_string(),
        )
        .expect("packet should write");
        let payload = consume_resume_error_payload(
            &format!(
                "dispatch packet contract invalid; dispatch packet `{}`",
                packet_path.display()
            ),
            "vida taskflow consume continue",
        );

        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["dispatch_packet_contract_invalid"])
        );
        assert_eq!(payload["artifact_refs"]["run_id"], "run-packet");
        assert_eq!(payload["artifact_refs"]["task_id"], "task-packet");
        assert!(payload["next_actions"][0]
            .as_str()
            .expect("next action should be text")
            .contains("vida taskflow packet repair --run-id run-packet --from-task task-packet"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn consume_resume_error_payload_does_not_read_outside_packet_refs() {
        let project_root = std::env::current_dir().expect("current dir");
        let outside_root = project_root.parent().expect("project parent").join(format!(
            "vida-taskflow-diagnostics-outside-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&outside_root).expect("outside root should create");
        let packet_path = outside_root.join("packet.json");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": "run-outside",
                "delivery_task_packet": {
                    "task_id": "task-outside"
                }
            })
            .to_string(),
        )
        .expect("outside packet should write");
        let payload = consume_resume_error_payload(
            &format!(
                "dispatch packet contract invalid; dispatch packet `{}`",
                packet_path.display()
            ),
            "vida taskflow consume continue",
        );

        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["dispatch_packet_contract_invalid"])
        );
        assert_eq!(
            payload["artifact_refs"]["dispatch_packet_path"],
            packet_path.display().to_string()
        );
        assert!(payload["artifact_refs"]["run_id"].is_null());
        assert!(payload["artifact_refs"]["task_id"].is_null());
        assert!(payload["next_actions"][0]
            .as_str()
            .expect("next action should be text")
            .contains("canonical task metadata"));

        let _ = std::fs::remove_dir_all(&outside_root);
    }
}
