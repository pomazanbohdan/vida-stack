use crate::{build_design_first_tracked_flow_bootstrap, RuntimeConsumptionLaneSelection};

pub(crate) fn runtime_tracked_flow_packet(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_id: &str,
    dispatch_target: &str,
) -> serde_json::Value {
    let tracked_packet_key = match dispatch_target {
        "spec-pack" => "spec_task",
        "work-pool-pack" => "work_pool_task",
        "dev-pack" => "dev_task",
        _ => "",
    };
    let tracked_flow_bootstrap = if role_selection.execution_plan["tracked_flow_bootstrap"]
        [tracked_packet_key]["task_id"]
        .as_str()
        .is_some()
    {
        role_selection.execution_plan["tracked_flow_bootstrap"].clone()
    } else {
        build_design_first_tracked_flow_bootstrap(&role_selection.request)
    };
    let tracked = tracked_flow_bootstrap
        .get(tracked_packet_key)
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "packet_id": format!("{run_id}::{dispatch_target}::tracked-flow"),
        "dispatch_target": dispatch_target,
        "tracked_packet_key": tracked_packet_key,
        "activation_semantics": "tracked_flow_materialization_only",
        "view_only": true,
        "executes_packet": false,
        "transfers_root_session_write_authority": false,
        "task_id": tracked["task_id"],
        "title": tracked["title"],
        "runtime": tracked["runtime"],
        "inspect_command": tracked["inspect_command"],
        "ensure_command": tracked["ensure_command"],
        "next_command": tracked["ensure_command"],
        "create_command": tracked["create_command"],
        "close_command": tracked["close_command"],
        "required": tracked["required"],
        "request": role_selection.request,
    })
}

pub(crate) fn runtime_packet_prompt(
    run_id: &str,
    dispatch_target: &str,
    handoff_runtime_role: &str,
    request_text: &str,
    orchestration_contract: &serde_json::Value,
) -> String {
    let replan_points = orchestration_contract["replanning"]["checkpoints"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let scope_guidance = if matches!(handoff_runtime_role, "orchestrator" | "root") {
        "Local orchestrator coding is forbidden without an explicit exception path.\nBefore any local write decision, re-check `vida status --json`, `vida taskflow recovery latest --json`, and `vida taskflow consume continue --json`.\nAfter any compact, continuity drop, or uncertainty about the active slice, re-read `AGENTS.md` and `AGENTS.sidecar.md`, rerun `vida orchestrator-init --json`, and restate `active_bounded_unit`, `why_this_unit`, and sequential-vs-parallel posture before continuing.\nCommentary, status output, and intermediate reports are visibility only; they never count as lawful pause boundaries by themselves.\nIf closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting.\nAfter any bounded result, green test, successful build, successful proof, runtime handoff, or delegated handoff/result, immediately bind and continue the already-evidenced next lawful continuation item in the same cycle instead of pausing at a summary.\nDo not self-select `ready_head[0]`, the first ready backlog item, or an adjacent sibling slice unless runtime evidence explicitly binds that bounded unit.\nIf continued-development intent is active but `vida status --json` or `vida orchestrator-init --json` does not expose explicit `active_bounded_unit`, `why_this_unit`, `primary_path`, and sequential-vs-parallel posture, fail closed to ambiguity instead of continuing implementation.\nFinding the patch location, reproducing a runtime defect, or hitting a worker timeout does not authorize root-session fallback; wait, reroute, or record the exception path first.\nAgent/thread limits, stale lane handles, or `not_found` carrier failures require saturation recovery first: inspect active lanes, synthesize completed returns, reclaim closeable lanes, and retry lawful `vida agent-init` dispatch before any local fallback is considered."
    } else {
        "This delegated lane does not hold root-session orchestration authority.\nYou are already inside the delegated lane activation; do not call `vida agent-init` again from this lane.\nDo not run root-only orchestration commands from this lane; leave status/recovery/continue surfaces to the orchestrator/root session.\nDo not treat commentary, status output, or an intermediate report from this lane as a completion boundary; keep working until the bounded handoff result or blocker is ready.\nDo not bind the next continuation item from this delegated lane; return bounded evidence so the orchestrator/root session can resume routing.\nIf you hit a bridge blocker, runtime timeout, or patch-location ambiguity, report the bounded blocker and wait for orchestrator reroute rather than reclaiming root-session fallback."
    };
    format!(
        "Packet run_id={run_id}\nTarget={dispatch_target}\nRuntime role={handoff_runtime_role}\nRoot session role=orchestrator\nExecution mode=delegated_orchestration_cycle\nCanonical delegated execution surface=vida agent-init\nThis packet activation view is not an execution receipt and does not transfer root-session write authority.\nIf the selected host/backend returns only this activation view without execution evidence, treat that as a bridge blocker, not as delegated work completion.\nIf a bounded read-only diagnostic path still exists, continue diagnosis to a code-level blocker or next bounded fix before asking the user to choose a route.\nHost subagent APIs are backend details only; do not substitute them for the project runtime's delegated lane contract.\nHost-local shell/edit capability is not a write-authority receipt.\nFirst substantive response: publish a concise plan before edits or implementation.\nIf the user explicitly ordered agent-first or parallel-agent execution, keep that routing sticky and do not silently substitute root-session implementation.\nUnder continued-development intent, stay in commentary/progress mode; final closure wording is forbidden unless the user explicitly asks to stop.\nDo not treat commentary, status output, an intermediate report, an intermediate status update, or “I have explained the result” as a lawful pause boundary.\nWhen recording task notes from shell, prefer `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.\n{scope_guidance}\nReplan checkpoints: {replan_points}\nGoal: execute only this bounded handoff and produce receipt-backed evidence.\nRequest: {request_text}"
    )
}

#[cfg(test)]
mod tests {
    use super::{runtime_packet_prompt, runtime_tracked_flow_packet};
    use crate::RuntimeConsumptionLaneSelection;
    use serde_json::json;

    #[test]
    fn delegated_lane_prompt_excludes_root_only_orchestration_commands() {
        let prompt = runtime_packet_prompt(
            "run-1",
            "coach",
            "coach",
            "continue the bounded review",
            &json!({
                "replanning": {
                    "checkpoints": ["after proof", "before close"]
                }
            }),
        );

        assert!(prompt
            .contains("This delegated lane does not hold root-session orchestration authority."));
        assert!(prompt.contains(
            "You are already inside the delegated lane activation; do not call `vida agent-init` again from this lane."
        ));
        assert!(prompt.contains("Do not run root-only orchestration commands"));
        assert!(prompt.contains(
            "Do not treat commentary, status output, or an intermediate report from this lane as a completion boundary"
        ));
        assert!(!prompt.contains("Before any local write decision, re-check `vida status --json`, `vida taskflow recovery latest --json`, and `vida taskflow consume continue --json`."));
        assert!(!prompt.contains("If closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting."));
    }

    #[test]
    fn orchestrator_prompt_retains_root_only_orchestration_guardrails() {
        let prompt = runtime_packet_prompt(
            "run-1",
            "dev-pack",
            "orchestrator",
            "continue the bounded orchestration cycle",
            &json!({
                "replanning": {
                    "checkpoints": ["after proof", "before close"]
                }
            }),
        );

        assert!(prompt.contains(
            "Local orchestrator coding is forbidden without an explicit exception path."
        ));
        assert!(prompt.contains("vida taskflow consume continue --json"));
        assert!(prompt.contains("re-read `AGENTS.md` and `AGENTS.sidecar.md`"));
        assert!(prompt.contains(
            "Commentary, status output, and intermediate reports are visibility only; they never count as lawful pause boundaries by themselves."
        ));
        assert!(prompt.contains(
            "After any bounded result, green test, successful build, successful proof, runtime handoff, or delegated handoff/result, immediately bind and continue the already-evidenced next lawful continuation item in the same cycle instead of pausing at a summary."
        ));
        assert!(prompt.contains(
            "restate `active_bounded_unit`, `why_this_unit`, and sequential-vs-parallel posture"
        ));
        assert!(!prompt
            .contains("This delegated lane does not hold root-session orchestration authority."));
    }

    #[test]
    fn runtime_tracked_flow_packet_marks_view_only_materialization_semantics() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("pbi_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("work-pool-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "tracked_flow_bootstrap": {
                    "work_pool_task": {
                        "task_id": "feature-x-work-pool",
                        "title": "Work-pool pack: Feature X",
                        "runtime": "vida taskflow",
                        "inspect_command": "vida task show feature-x-work-pool --json",
                        "ensure_command": "vida task ensure feature-x-work-pool \"Work-pool pack: Feature X\" --type task --status open --json",
                        "create_command": "vida task create feature-x-work-pool \"Work-pool pack: Feature X\" --type task --status open --json",
                        "close_command": "vida task close feature-x-work-pool --reason 'closed' --json",
                        "required": true
                    }
                }
            }),
            reason: "test".to_string(),
        };

        let packet = runtime_tracked_flow_packet(&role_selection, "run-1", "work-pool-pack");
        assert_eq!(
            packet["activation_semantics"],
            "tracked_flow_materialization_only"
        );
        assert_eq!(packet["view_only"], true);
        assert_eq!(packet["executes_packet"], false);
        assert_eq!(packet["transfers_root_session_write_authority"], false);
    }
}
