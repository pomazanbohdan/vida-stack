use crate::{build_design_first_tracked_flow_bootstrap, RuntimeConsumptionLaneSelection};

fn json_string(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn json_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn push_line(lines: &mut Vec<String>, label: &str, value: Option<String>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {value}"));
    }
}

fn push_list(lines: &mut Vec<String>, label: &str, values: Vec<String>) {
    if values.is_empty() {
        return;
    }
    lines.push(format!("{label}: {}", values.join("; ")));
}

fn active_packet<'a>(
    packet_template_kind: &str,
    packet: &'a serde_json::Value,
) -> Option<&'a serde_json::Value> {
    packet
        .get(packet_template_kind)
        .filter(|value| !value.is_null())
}

pub(crate) fn runtime_packet_request_text(
    packet_template_kind: &str,
    packet: &serde_json::Value,
) -> Option<String> {
    if let Some(request_text) = json_string(packet.get("request_text")) {
        return Some(request_text);
    }

    let active_packet = active_packet(packet_template_kind, packet)?;
    let mut lines = Vec::new();
    match packet_template_kind {
        "coach_review_packet" => {
            push_line(
                &mut lines,
                "review_goal",
                json_string(active_packet.get("review_goal")),
            );
            push_line(
                &mut lines,
                "review_subject",
                json_string(active_packet.get("review_subject")),
            );
            push_line(
                &mut lines,
                "blocking_question",
                json_string(active_packet.get("blocking_question")),
            );
            push_line(
                &mut lines,
                "proof_target",
                json_string(active_packet.get("proof_target")),
            );
            push_list(
                &mut lines,
                "expected_output",
                json_string_array(active_packet.get("expected_output")),
            );
            push_list(
                &mut lines,
                "review_focus",
                json_string_array(active_packet.get("review_focus")),
            );
            push_list(
                &mut lines,
                "read_only_paths",
                json_string_array(active_packet.get("read_only_paths")),
            );
        }
        "verifier_proof_packet" => {
            push_line(
                &mut lines,
                "proof_goal",
                json_string(active_packet.get("proof_goal")),
            );
            push_line(
                &mut lines,
                "blocking_question",
                json_string(active_packet.get("blocking_question")),
            );
            push_line(
                &mut lines,
                "verification_command",
                json_string(active_packet.get("verification_command")),
            );
            push_line(
                &mut lines,
                "proof_target",
                json_string(active_packet.get("proof_target")),
            );
        }
        "delivery_task_packet" | "execution_block_packet" => {
            push_line(&mut lines, "goal", json_string(active_packet.get("goal")));
            push_line(
                &mut lines,
                "blocking_question",
                json_string(active_packet.get("blocking_question")),
            );
            push_line(
                &mut lines,
                "proof_target",
                json_string(active_packet.get("proof_target")),
            );
            push_list(
                &mut lines,
                "scope_in",
                json_string_array(active_packet.get("scope_in")),
            );
            push_list(
                &mut lines,
                "definition_of_done",
                json_string_array(active_packet.get("definition_of_done")),
            );
        }
        "escalation_packet" => {
            push_line(
                &mut lines,
                "decision_needed",
                json_string(active_packet.get("decision_needed")),
            );
            push_line(
                &mut lines,
                "blocking_question",
                json_string(active_packet.get("blocking_question")),
            );
            push_list(
                &mut lines,
                "options",
                json_string_array(active_packet.get("options")),
            );
            push_list(
                &mut lines,
                "constraints",
                json_string_array(active_packet.get("constraints")),
            );
        }
        "tracked_flow_packet" => {
            push_line(&mut lines, "title", json_string(active_packet.get("title")));
            push_line(
                &mut lines,
                "task_id",
                json_string(active_packet.get("task_id")),
            );
            push_line(
                &mut lines,
                "ensure_command",
                json_string(active_packet.get("ensure_command")),
            );
            push_line(
                &mut lines,
                "request",
                json_string(active_packet.get("request")),
            );
        }
        _ => {}
    }

    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn prompt_has_empty_request_tail(prompt: &str) -> bool {
    prompt.trim_end().ends_with("Request:")
}

pub(crate) fn runtime_packet_prompt_prelaunch_blocker(
    packet: &serde_json::Value,
) -> Option<String> {
    let packet_kind = packet
        .get("packet_kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !matches!(
        packet_kind,
        "runtime_dispatch_packet" | "runtime_downstream_dispatch_packet"
    ) {
        return None;
    }

    let packet_template_kind = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let request_text = runtime_packet_request_text(packet_template_kind, packet);
    let prompt = packet
        .get("prompt")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if prompt.is_none() && request_text.is_none() {
        return Some(
            "dispatch packet has no non-empty prompt and no structured request body".to_string(),
        );
    }
    if prompt.is_some_and(prompt_has_empty_request_tail) && request_text.is_none() {
        return Some(
            "dispatch packet prompt ends with an empty Request field and no structured request body"
                .to_string(),
        );
    }
    None
}

pub(crate) fn runtime_packet_prompt_from_packet(
    packet: &serde_json::Value,
    fallback_dispatch_packet_path: &str,
) -> String {
    let packet_template_kind = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let request_text = runtime_packet_request_text(packet_template_kind, packet);
    let prompt = packet
        .get("prompt")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(prompt) = prompt.filter(|prompt| !prompt_has_empty_request_tail(prompt)) {
        return prompt.to_string();
    }
    if let Some(request_text) = request_text.as_deref() {
        if let Some(prompt) = prompt {
            return format!("{} {}", prompt.trim_end(), request_text);
        }
        let run_id = packet
            .get("run_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown-run");
        let dispatch_target = packet
            .get("downstream_dispatch_target")
            .or_else(|| packet.get("dispatch_target"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown-target");
        let runtime_role = packet
            .get("activation_runtime_role")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("worker");
        return runtime_packet_prompt(
            run_id,
            dispatch_target,
            runtime_role,
            request_text,
            &packet["orchestration_contract"],
        );
    }

    format!(
        "Read and execute the VIDA dispatch packet at {}. Return one bounded result that follows the packet.",
        fallback_dispatch_packet_path
    )
}

fn delegated_lane_prompt_guidance(
    dispatch_target: &str,
    handoff_runtime_role: &str,
) -> &'static str {
    let dispatch_target = dispatch_target.trim();
    let handoff_runtime_role = handoff_runtime_role.trim();
    let is_review_or_proof_lane = matches!(handoff_runtime_role, "coach" | "verifier" | "prover")
        || matches!(
            dispatch_target,
            "coach" | "review" | "verification" | "verifier" | "prover"
        )
        || dispatch_target.contains("coach")
        || dispatch_target.contains("review")
        || dispatch_target.contains("verification");

    if is_review_or_proof_lane {
        "Review/proof lane contract: do not edit files, do not create commits, and do not keep exploring after the decision is supported.\nInspect only packet-provided read-only paths, dispatch result artifacts, and focused proof evidence when needed.\nReturn one bounded handoff result with: decision=approve|rework|blocker, checked_evidence, findings, risks, next_required_action.\nKeep the handoff concise and receipt-oriented."
    } else {
        "First substantive response: publish a concise plan before edits or implementation."
    }
}

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
    let is_orchestrator_lane = matches!(handoff_runtime_role, "orchestrator" | "root");
    let lane_guidance = if is_orchestrator_lane {
        "First substantive response: publish a concise plan before edits or implementation."
    } else {
        delegated_lane_prompt_guidance(dispatch_target, handoff_runtime_role)
    };
    let scope_guidance = if is_orchestrator_lane {
        "Local orchestrator coding is forbidden without an explicit exception path.\nBefore any local write decision, re-check `vida status`, `vida taskflow recovery latest`, and `vida taskflow consume continue`.\nAfter any compact, continuity drop, or uncertainty about the active slice, re-read `AGENTS.md` and `AGENTS.sidecar.md`, rerun `vida orchestrator-init`, and restate `active_bounded_unit`, `why_this_unit`, and sequential-vs-parallel posture before continuing.\nCommentary, status output, and intermediate reports are visibility only; they never count as lawful pause boundaries by themselves.\nIf closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting.\nAfter any bounded result, green test, successful build, successful proof, runtime handoff, or delegated handoff/result, immediately bind and continue the already-evidenced next lawful continuation item in the same cycle instead of pausing at a summary.\nDo not self-select `ready_head[0]`, the first ready backlog item, or an adjacent sibling slice unless runtime evidence explicitly binds that bounded unit.\nIf continued-development intent is active but `vida status` or `vida orchestrator-init` does not expose explicit `active_bounded_unit`, `why_this_unit`, `primary_path`, and sequential-vs-parallel posture, fail closed to ambiguity instead of continuing implementation.\nFinding the patch location, reproducing a runtime defect, or hitting a worker timeout does not authorize root-session fallback; wait, reroute, or record the exception path first.\nAgent/thread limits, stale lane handles, or `not_found` carrier failures require saturation recovery first: inspect active lanes, synthesize completed returns, reclaim closeable lanes, and retry lawful `vida agent-init` dispatch before any local fallback is considered."
    } else {
        "This delegated lane does not hold root-session orchestration authority.\nYou are already inside the delegated lane activation; do not call `vida agent-init` again from this lane.\nDo not run root-only orchestration commands from this lane; leave status/recovery/continue surfaces to the orchestrator/root session.\nDo not treat commentary, status output, or an intermediate report from this lane as a completion boundary; keep working until the bounded handoff result or blocker is ready.\nDo not bind the next continuation item from this delegated lane; return bounded evidence so the orchestrator/root session can resume routing.\nIf you hit a bridge blocker, runtime timeout, or patch-location ambiguity, report the bounded blocker and wait for orchestrator reroute rather than reclaiming root-session fallback."
    };
    format!(
        "Packet run_id={run_id}\nTarget={dispatch_target}\nRuntime role={handoff_runtime_role}\nRoot session role=orchestrator\nExecution mode=delegated_orchestration_cycle\nCanonical delegated execution surface=vida agent-init\nThis packet activation view is not an execution receipt and does not transfer root-session write authority.\nIf the selected host/backend returns only this activation view without execution evidence, treat that as a bridge blocker, not as delegated work completion.\nIf a bounded read-only diagnostic path still exists, continue diagnosis to a code-level blocker or next bounded fix before asking the user to choose a route.\nHost subagent APIs are backend details only; do not substitute them for the project runtime's delegated lane contract.\nWhen project default orchestration, runtime policy, or the user explicitly orders agent-first or parallel-agent execution, keep that routing sticky and do not silently substitute root-session implementation.\nFor configured host subagent adapters, project default orchestration, runtime policy, the current bounded VIDA dispatch packet, or a host bridge execution request satisfies host-tool explicit delegation permission; do not wait for a second chat-only permission phrase before launching configured carriers.\nThat host-tool permission is scoped to this run, runtime role, packet, owned paths, and receipt mode; it does not weaken `vida agent-init` authority, TaskFlow binding, exception takeover, receipt-backed closure rules, or root write guard boundaries.\nHost-local shell/edit capability is not a write-authority receipt.\n{lane_guidance}\nUnder continued-development intent, stay in commentary/progress mode; final closure wording is forbidden unless the user explicitly asks to stop.\nDo not treat commentary, status output, an intermediate report, an intermediate status update, or “I have explained the result” as a lawful pause boundary.\nWhen recording task notes from shell, prefer `vida task update <task-id> --notes-file <path>` over inline shell quoting for complex text.\n{scope_guidance}\nReplan checkpoints: {replan_points}\nGoal: execute only this bounded handoff and produce receipt-backed evidence.\nRequest: {request_text}"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        runtime_packet_prompt, runtime_packet_prompt_from_packet, runtime_packet_request_text,
        runtime_tracked_flow_packet,
    };
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
        assert!(prompt.contains("Review/proof lane contract: do not edit files"));
        assert!(prompt.contains("decision=approve|rework|blocker"));
        assert!(!prompt.contains(
            "First substantive response: publish a concise plan before edits or implementation."
        ));
        assert!(!prompt.contains("Before any local write decision, re-check `vida status`, `vida taskflow recovery latest`, and `vida taskflow consume continue`."));
        assert!(!prompt.contains("vida status --json"));
        assert!(!prompt.contains("vida orchestrator-init --json"));
        assert!(!prompt.contains("If closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting."));
    }

    #[test]
    fn coach_review_packet_request_text_is_synthesized_from_structured_fields() {
        let packet = json!({
            "packet_template_kind": "coach_review_packet",
            "coach_review_packet": {
                "review_goal": "Judge the bounded result",
                "review_subject": "bounded implementation result",
                "blocking_question": "Can this proceed?",
                "proof_target": "implementation evidence",
                "expected_output": ["decision=approve|rework|blocker", "checked_evidence"],
                "review_focus": ["scope", "proof"],
                "read_only_paths": [".vida/data/state/runtime-consumption"]
            }
        });

        let request = runtime_packet_request_text("coach_review_packet", &packet)
            .expect("coach request should synthesize from structured packet");

        assert!(request.contains("review_goal: Judge the bounded result"));
        assert!(request.contains("blocking_question: Can this proceed?"));
        assert!(
            request.contains("expected_output: decision=approve|rework|blocker; checked_evidence")
        );
    }

    #[test]
    fn stale_empty_request_prompt_is_repaired_from_active_packet_body() {
        let packet = json!({
            "run_id": "run-1",
            "downstream_dispatch_target": "coach",
            "activation_runtime_role": "coach",
            "packet_template_kind": "coach_review_packet",
            "prompt": "Packet run_id=run-1\nTarget=coach\nRequest: ",
            "coach_review_packet": {
                "review_goal": "Review the result",
                "blocking_question": "Is there receipt-backed evidence?",
                "expected_output": ["decision=approve|rework|blocker"]
            },
            "orchestration_contract": {
                "replanning": {
                    "checkpoints": ["after review"]
                }
            }
        });

        let prompt = runtime_packet_prompt_from_packet(&packet, "/tmp/downstream.json");

        assert!(prompt.contains("Request: review_goal: Review the result"));
        assert!(prompt.contains("blocking_question: Is there receipt-backed evidence?"));
        assert!(!prompt.trim_end().ends_with("Request:"));
    }

    #[test]
    fn structured_packet_without_prompt_or_request_fails_prelaunch_validation() {
        let packet = json!({
            "packet_kind": "runtime_downstream_dispatch_packet",
            "packet_template_kind": "coach_review_packet",
            "coach_review_packet": {}
        });

        let blocker = super::runtime_packet_prompt_prelaunch_blocker(&packet)
            .expect("empty structured packet should block external launch");

        assert!(blocker.contains("no non-empty prompt"));
    }

    #[test]
    fn implementation_delegated_lane_keeps_bounded_plan_guidance() {
        let prompt = runtime_packet_prompt(
            "run-1",
            "implementer",
            "worker",
            "continue the bounded implementation",
            &json!({
                "replanning": {
                    "checkpoints": ["after proof", "before close"]
                }
            }),
        );

        assert!(prompt.contains(
            "First substantive response: publish a concise plan before edits or implementation."
        ));
        assert!(!prompt.contains("decision=approve|rework|blocker"));
    }

    #[test]
    fn runtime_packet_prompt_treats_default_orchestration_as_host_bridge_permission() {
        let prompt = runtime_packet_prompt(
            "run-1",
            "implementer",
            "worker",
            "continue the bounded implementation",
            &json!({
                "replanning": {
                    "checkpoints": ["after proof", "before close"]
                }
            }),
        );

        assert!(prompt.contains(
            "When project default orchestration, runtime policy, or the user explicitly orders agent-first or parallel-agent execution"
        ));
        assert!(prompt.contains(
            "For configured host subagent adapters, project default orchestration, runtime policy, the current bounded VIDA dispatch packet, or a host bridge execution request satisfies host-tool explicit delegation permission"
        ));
        assert!(prompt.contains(
            "do not wait for a second chat-only permission phrase before launching configured carriers"
        ));
        assert!(prompt.contains(
            "That host-tool permission is scoped to this run, runtime role, packet, owned paths, and receipt mode"
        ));
        assert!(prompt.contains("receipt-backed closure rules, or root write guard boundaries"));
        assert!(!prompt
            .contains("If the user explicitly ordered agent-first or parallel-agent execution"));
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
        assert!(prompt.contains("vida taskflow consume continue"));
        assert!(!prompt.contains("vida taskflow consume continue --json"));
        assert!(!prompt.contains("vida status --json"));
        assert!(!prompt.contains("vida orchestrator-init --json"));
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
                        "inspect_command": "vida task show feature-x-work-pool",
                        "ensure_command": "vida task ensure feature-x-work-pool \"Work-pool pack: Feature X\" --type task --status open",
                        "create_command": "vida task create feature-x-work-pool \"Work-pool pack: Feature X\" --type task --status open",
                        "close_command": "vida task close feature-x-work-pool --reason 'closed'",
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
