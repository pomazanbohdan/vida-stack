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
    format!(
        "Packet run_id={run_id}\nTarget={dispatch_target}\nRuntime role={handoff_runtime_role}\nRoot session role=orchestrator\nExecution mode=delegated_orchestration_cycle\nCanonical delegated execution surface=vida agent-init\nThis packet activation view is not an execution receipt and does not transfer root-session write authority.\nIf the selected host/backend returns only this activation view without execution evidence, treat that as a bridge blocker, not as delegated work completion.\nIf a bounded read-only diagnostic path still exists, continue diagnosis to a code-level blocker or next bounded fix before asking the user to choose a route.\nHost subagent APIs are backend details only; do not substitute them for the project runtime's delegated lane contract.\nHost-local shell/edit capability is not a write-authority receipt.\nFirst substantive response: publish a concise plan before edits or implementation.\nLocal orchestrator coding is forbidden without an explicit exception path.\nBefore any local write decision, re-check `vida status --json`, `vida taskflow recovery latest --json`, and `vida taskflow consume continue --json`.\nIf the user explicitly ordered agent-first or parallel-agent execution, keep that routing sticky and do not silently substitute root-session implementation.\nUnder continued-development intent, stay in commentary/progress mode; final closure wording is forbidden unless the user explicitly asks to stop.\nDo not treat commentary, an intermediate status update, or “I have explained the result” as a lawful pause boundary.\nIf closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting.\nAfter any bounded result, green test, successful build, or delegated handoff, immediately bind the next lawful continuation item instead of pausing at a summary.\nWhen recording task notes from shell, prefer `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.\nFinding the patch location, reproducing a runtime defect, or hitting a worker timeout does not authorize root-session fallback; wait, reroute, or record the exception path first.\nAgent/thread limits, stale lane handles, or `not_found` carrier failures require saturation recovery first: inspect active lanes, synthesize completed returns, reclaim closeable lanes, and retry lawful `vida agent-init` dispatch before any local fallback is considered.\nReplan checkpoints: {replan_points}\nGoal: execute only this bounded handoff and produce receipt-backed evidence.\nRequest: {request_text}"
    )
}
