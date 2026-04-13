use crate::runtime_contract_vocab::{
    RUNTIME_ROLE_COACH, RUNTIME_ROLE_SOLUTION_ARCHITECT, RUNTIME_ROLE_VERIFIER,
    TASK_CLASS_ARCHITECTURE, TASK_CLASS_COACH, TASK_CLASS_VERIFICATION,
};

fn runtime_delivery_packet_id(run_id: &str, dispatch_target: &str) -> String {
    format!("{run_id}::{dispatch_target}::delivery")
}

fn runtime_execution_block_packet_id(run_id: &str, dispatch_target: &str) -> String {
    format!("{run_id}::{dispatch_target}::execution-block")
}

fn runtime_coach_review_packet_id(run_id: &str, dispatch_target: &str) -> String {
    format!("{run_id}::{dispatch_target}::coach-review")
}

fn runtime_verifier_proof_packet_id(run_id: &str, dispatch_target: &str) -> String {
    format!("{run_id}::{dispatch_target}::verifier-proof")
}

fn runtime_escalation_packet_id(run_id: &str, dispatch_target: &str) -> String {
    format!("{run_id}::{dispatch_target}::escalation")
}

fn runtime_delivery_source_packet_id(run_id: &str, dispatch_target: &str) -> String {
    runtime_delivery_packet_id(run_id, dispatch_target)
}

fn runtime_implementer_delivery_source_packet_id(run_id: &str) -> String {
    runtime_delivery_source_packet_id(run_id, "implementer")
}

fn trim_move_request_scope_path(segment: &str) -> String {
    segment
        .trim()
        .trim_matches(|ch: char| {
            ch.is_whitespace() || matches!(ch, '`' | '"' | '\'' | ',' | ';' | ':')
        })
        .trim_end_matches('.')
        .to_string()
}

pub(crate) fn single_task_move_scope_paths(request_text: &str) -> Option<Vec<String>> {
    let lowered = request_text.to_ascii_lowercase();
    let move_start = lowered.find("move ")?;
    let from_start = lowered[move_start + "move ".len()..]
        .find(" from ")
        .map(|offset| move_start + "move ".len() + offset)?;
    let source_start = from_start + " from ".len();
    let into_start = lowered[source_start..]
        .find(" into ")
        .map(|offset| source_start + offset)?;
    let source_path = trim_move_request_scope_path(&request_text[source_start..into_start]);
    if source_path.is_empty() {
        return None;
    }

    let destination_start = into_start + " into ".len();
    let destination_segment = &request_text[destination_start..];
    let destination_lowered = &lowered[destination_start..];
    let mut destination_end = destination_segment.len();
    for marker in [
        "\n",
        ";",
        " keep scope",
        " keep ",
        " proof target",
        " proof ",
        " after ",
    ] {
        if let Some(offset) = destination_lowered.find(marker) {
            destination_end = destination_end.min(offset);
        }
    }
    let destination_path = trim_move_request_scope_path(&destination_segment[..destination_end]);
    if destination_path.is_empty() {
        return None;
    }

    Some(vec![source_path, destination_path])
}

pub(crate) fn runtime_delivery_task_packet(
    run_id: &str,
    dispatch_target: &str,
    handoff_runtime_role: &str,
    handoff_task_class: &str,
    closure_class: &str,
    request_text: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": runtime_delivery_packet_id(run_id, dispatch_target),
        "backlog_id": run_id,
        "release_slice": "none",
        "owner": "taskflow",
        "closure_class": closure_class,
        "goal": format!("Execute bounded `{dispatch_target}` handoff for the active runtime request"),
        "non_goals": [
            "unbounded repository-wide rewrites",
            "out-of-scope taskflow state mutation"
        ],
        "scope_in": [
            format!("dispatch_target:{dispatch_target}"),
            format!("runtime_role:{handoff_runtime_role}")
        ],
        "scope_out": [
            "mutation outside bounded packet scope",
            "closure without recorded handoff evidence"
        ],
        "owned_paths": single_task_move_scope_paths(request_text).unwrap_or_default(),
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "inputs": [
            "role_selection_full",
            "run_graph_bootstrap",
            "taskflow_handoff_plan"
        ],
        "outputs": [
            "dispatch_result_artifact",
            "updated_run_graph_dispatch_receipt"
        ],
        "definition_of_done": [
            format!("`{dispatch_target}` handoff produces a bounded runtime result artifact"),
            "dispatch receipt and downstream preview are refreshed consistently"
        ],
        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
        "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
        "active_skills": "no_applicable_skill",
        "stop_rules": [
            "stop after writing bounded dispatch result or explicit blocker",
            "do not widen scope beyond the active packet target"
        ],
        "blocking_question": format!("What is the next bounded action required for `{dispatch_target}`?"),
        "handoff_runtime_role": handoff_runtime_role,
        "handoff_task_class": handoff_task_class,
        "handoff_selection": "runtime_selected_tier",
        "request_excerpt": request_text.chars().take(240).collect::<String>(),
    })
}

pub(crate) fn runtime_execution_block_packet(
    run_id: &str,
    dispatch_target: &str,
    handoff_runtime_role: &str,
    handoff_task_class: &str,
    closure_class: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": runtime_execution_block_packet_id(run_id, dispatch_target),
        "parent_packet_id": runtime_delivery_packet_id(run_id, dispatch_target),
        "backlog_id": run_id,
        "owner": "taskflow",
        "closure_class": closure_class,
        "goal": format!("Resolve bounded execution blocker for `{dispatch_target}`"),
        "scope_in": [
            format!("dispatch_target:{dispatch_target}")
        ],
        "scope_out": [
            "new feature scope without bounded packet update"
        ],
        "owned_paths": [],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "definition_of_done": [
            "bounded blocker is resolved with receipt-backed evidence"
        ],
        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
        "proof_target": "runtime receipt evidence that blocker is resolved or escalated",
        "active_skills": "no_applicable_skill",
        "stop_rules": [
            "stop once blocker resolution evidence is recorded"
        ],
        "blocking_question": format!("Which explicit blocker prevents closing `{dispatch_target}` now?"),
        "handoff_runtime_role": handoff_runtime_role,
        "handoff_task_class": handoff_task_class,
        "handoff_selection": "runtime_selected_tier"
    })
}

pub(crate) fn runtime_coach_review_packet(
    run_id: &str,
    dispatch_target: &str,
    proof_target: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": runtime_coach_review_packet_id(run_id, dispatch_target),
        "source_packet_id": runtime_implementer_delivery_source_packet_id(run_id),
        "review_goal": format!("Judge whether `{dispatch_target}` remains aligned with the approved bounded packet, acceptance criteria, and definition of done"),
        "owned_paths": [],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "definition_of_done": [
            "coach review returns bounded approval-forward or bounded rework evidence"
        ],
        "proof_target": proof_target,
        "active_skills": "no_applicable_skill",
        "review_focus": [
            "spec_conformance",
            "acceptance_criteria_alignment",
            "bounded_scope_drift"
        ],
        "blocking_question": format!("Does `{dispatch_target}` match the approved bounded contract cleanly enough to proceed?"),
        "handoff_runtime_role": RUNTIME_ROLE_COACH,
        "handoff_task_class": TASK_CLASS_COACH,
        "handoff_selection": "runtime_selected_tier",
    })
}

pub(crate) fn runtime_verifier_proof_packet(
    run_id: &str,
    dispatch_target: &str,
    proof_target: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": runtime_verifier_proof_packet_id(run_id, dispatch_target),
        "source_packet_id": runtime_implementer_delivery_source_packet_id(run_id),
        "proof_goal": format!("Independently verify bounded closure readiness for `{dispatch_target}`"),
        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
        "proof_target": proof_target,
        "owned_paths": [],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "active_skills": "no_applicable_skill",
        "blocking_question": format!("What proof is still missing before `{dispatch_target}` can close?"),
        "handoff_runtime_role": RUNTIME_ROLE_VERIFIER,
        "handoff_task_class": TASK_CLASS_VERIFICATION,
        "handoff_selection": "runtime_selected_tier",
    })
}

pub(crate) fn runtime_escalation_packet(run_id: &str, dispatch_target: &str) -> serde_json::Value {
    serde_json::json!({
        "packet_id": runtime_escalation_packet_id(run_id, dispatch_target),
        "source_packet_id": runtime_delivery_source_packet_id(run_id, dispatch_target),
        "conflict_type": "architecture",
        "decision_needed": format!("Resolve the bounded architecture-preparation or escalation decision for `{dispatch_target}`"),
        "options": [
            "approve current bounded route",
            "reshape bounded handoff",
            "block execution pending architectural clarification"
        ],
        "constraints": [
            "preserve one bounded packet owner",
            "do not widen scope without a new bounded packet"
        ],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "active_skills": "no_applicable_skill",
        "blocking_question": format!("Which architectural decision is required before `{dispatch_target}` can proceed coherently?"),
        "handoff_runtime_role": RUNTIME_ROLE_SOLUTION_ARCHITECT,
        "handoff_task_class": TASK_CLASS_ARCHITECTURE,
        "handoff_selection": "runtime_selected_tier",
    })
}
