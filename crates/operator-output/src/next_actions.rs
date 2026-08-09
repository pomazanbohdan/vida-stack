use crate::command_text::human_command;
use serde::{Deserialize, Serialize};

pub const NEXT_ACTION_REDUCER_SCHEMA_VERSION: &str = "next-action-reducer-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionKind {
    Bind,
    Continue,
    Execute,
    Inspect,
    Recompute,
    Recover,
}

impl NextActionKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Bind => "bind",
            Self::Continue => "continue",
            Self::Execute => "execute",
            Self::Inspect => "inspect",
            Self::Recompute => "recompute",
            Self::Recover => "recover",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedNextAction {
    pub action_id: String,
    pub kind: NextActionKind,
    pub command: String,
    pub expected_output: Vec<String>,
    pub approval_required: bool,
    pub artifact_refs: serde_json::Value,
    pub surface: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NextActionUnit {
    pub id: String,
    pub title: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NextActionLane {
    pub id: Option<String>,
    pub role: Option<String>,
    pub task_class: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NextActionReferences {
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub packet_path: Option<String>,
    pub result_path: Option<String>,
    pub receipt_path: Option<String>,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextActionBlocker {
    pub code: String,
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AfterSuccessContract {
    pub next_command: String,
    pub requires_receipt: bool,
    pub invariant: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextActionReducerInput {
    pub status: String,
    pub current_unit: Option<NextActionUnit>,
    pub lane: Option<NextActionLane>,
    pub next_action: Option<TypedNextAction>,
    pub projection_cache: Option<serde_json::Value>,
    pub packet_refs: NextActionReferences,
    pub context_refs: NextActionReferences,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
    pub after_success: Option<AfterSuccessContract>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextActionReducerOutput {
    pub schema_version: String,
    pub status: String,
    pub current_unit: Option<NextActionUnit>,
    pub lane: Option<NextActionLane>,
    pub next_action: Option<TypedNextAction>,
    pub projection_cache: Option<serde_json::Value>,
    pub packet_refs: NextActionReferences,
    pub context_refs: NextActionReferences,
    pub blockers: Vec<NextActionBlocker>,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
    pub after_success: AfterSuccessContract,
}

pub fn reduce_next_action(input: NextActionReducerInput) -> NextActionReducerOutput {
    let blocker_codes = input
        .blocker_codes
        .into_iter()
        .map(|code| code.trim().to_ascii_lowercase())
        .filter(|code| !code.is_empty())
        .collect::<Vec<_>>();
    let next_actions = input
        .next_actions
        .into_iter()
        .map(|action| action.trim().to_string())
        .filter(|action| !action.is_empty())
        .collect::<Vec<_>>();
    let blockers = blocker_codes
        .iter()
        .enumerate()
        .map(|(index, code)| NextActionBlocker {
            code: code.clone(),
            next_action: next_actions.get(index).cloned(),
        })
        .collect();
    let after_success = input.after_success.unwrap_or_else(|| AfterSuccessContract {
        next_command: "vida task next".to_string(),
        requires_receipt: input
            .packet_refs
            .packet_path
            .as_ref()
            .is_some_and(|path| !path.trim().is_empty()),
        invariant: "reconcile the receipt-backed result before selecting another bounded unit"
            .to_string(),
    });

    NextActionReducerOutput {
        schema_version: NEXT_ACTION_REDUCER_SCHEMA_VERSION.to_string(),
        status: input.status,
        current_unit: input.current_unit,
        lane: input.lane,
        next_action: input.next_action,
        projection_cache: input.projection_cache,
        packet_refs: input.packet_refs,
        context_refs: input.context_refs,
        blockers,
        blocker_codes,
        next_actions,
        after_success,
    }
}

fn non_empty_string(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn first_object(value: &serde_json::Value, path: &[&str]) -> Option<serde_json::Value> {
    let value = path.iter().try_fold(value, |value, key| value.get(*key))?;
    value
        .as_array()
        .and_then(|values| values.first())
        .cloned()
        .or_else(|| value.is_object().then(|| value.clone()))
}

fn first_string(value: &serde_json::Value, paths: &[&[&str]]) -> Option<String> {
    paths.iter().find_map(|path| {
        let value = path
            .iter()
            .fold(value.clone(), |value: serde_json::Value, key: &&str| {
                value.get(*key).cloned().unwrap_or(serde_json::Value::Null)
            });
        non_empty_string(Some(&value))
    })
}

fn first_bool(value: &serde_json::Value, paths: &[&[&str]]) -> Option<bool> {
    paths.iter().find_map(|path| {
        let value = path
            .iter()
            .fold(value.clone(), |value: serde_json::Value, key: &&str| {
                value.get(*key).cloned().unwrap_or(serde_json::Value::Null)
            });
        value.as_bool()
    })
}

fn string_list(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(value) if value.is_array() => value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        Some(value) => non_empty_string(Some(value)).into_iter().collect(),
        None => Vec::new(),
    }
}

fn expected_output_from_projection(value: &serde_json::Value) -> Vec<String> {
    let expected_output = value
        .get("next_action")
        .and_then(|action| action.get("expected_output"))
        .or_else(|| value.get("expected_output"))
        .or_else(|| {
            value
                .get("flow_projection")
                .and_then(|projection| projection.get("expected_output"))
        });
    let expected_output = string_list(expected_output);
    if !expected_output.is_empty() {
        return expected_output;
    }
    value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(|status| vec![format!("status={status}")])
        .unwrap_or_else(|| vec!["operator_projection".to_string()])
}

fn approval_required_from_projection(value: &serde_json::Value) -> bool {
    first_bool(
        value,
        &[
            &["next_action", "approval_required"],
            &["approval_required"],
            &["requires_user_approval"],
            &["flow_projection", "current_step", "approval_required"],
            &["flow_projection", "current_step", "requires_user_approval"],
        ],
    )
    .or_else(|| {
        value
            .get("selected_lanes")
            .and_then(serde_json::Value::as_array)
            .and_then(|lanes| lanes.first())
            .and_then(|lane| lane.get("requires_user_approval"))
            .and_then(serde_json::Value::as_bool)
    })
    .unwrap_or(false)
}

fn artifact_refs_from_projection(value: &serde_json::Value) -> serde_json::Value {
    if let Some(artifact_refs) = value.get("artifact_refs").filter(|value| !value.is_null()) {
        return artifact_refs.clone();
    }
    if let Some(artifacts) = value
        .get("packet_materialization")
        .and_then(|materialization| materialization.get("artifacts"))
        .filter(|value| !value.is_null())
    {
        return serde_json::json!({"packet_materialization": artifacts});
    }
    serde_json::json!({})
}

fn action_id(kind: &NextActionKind, command: &str, surface: &str) -> String {
    let raw = format!("next_action_{}_{}_{}", kind.as_str(), surface, command);
    let mut normalized = String::with_capacity(raw.len());
    for character in raw.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
        } else if !normalized.ends_with('_') {
            normalized.push('_');
        }
    }
    normalized.trim_end_matches('_').to_string()
}

fn infer_action_kind(command: &str, surface: &str) -> NextActionKind {
    let haystack = format!("{command} {surface}").to_ascii_lowercase();
    if haystack.contains("bind") {
        NextActionKind::Bind
    } else if haystack.contains("continue") {
        NextActionKind::Continue
    } else if haystack.contains("recover") || haystack.contains("retire") {
        NextActionKind::Recover
    } else if haystack.contains("recompute") || haystack.contains("refresh") {
        NextActionKind::Recompute
    } else if haystack.contains("dispatch") || haystack.contains("execute") {
        NextActionKind::Execute
    } else {
        NextActionKind::Inspect
    }
}

fn action_from_projection(value: &serde_json::Value) -> Option<TypedNextAction> {
    let action = value.get("next_action").filter(|value| value.is_object());
    let command = first_string(
        value,
        &[
            &["next_action", "command"],
            &["flow_projection", "current_step", "dispatch_command"],
            &[
                "packet_materialization",
                "artifacts",
                "0",
                "agent_init_execute_command",
            ],
        ],
    )?;
    let surface = action
        .and_then(|value| non_empty_string(value.get("surface")))
        .or_else(|| {
            first_string(
                value,
                &[&["recommended_surface"], &["dispatch", "dispatch_surface"]],
            )
        })
        .unwrap_or_else(|| "vida task next".to_string());
    let reason = action
        .and_then(|value| non_empty_string(value.get("reason")))
        .or_else(|| {
            value
                .get("next_actions")
                .and_then(serde_json::Value::as_array)
                .and_then(|actions| actions.first())
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| {
            "continue the current bounded unit from authoritative runtime evidence".to_string()
        });
    let kind = action
        .and_then(|value| non_empty_string(value.get("kind")))
        .and_then(|kind| match kind.as_str() {
            "bind" => Some(NextActionKind::Bind),
            "continue" => Some(NextActionKind::Continue),
            "execute" => Some(NextActionKind::Execute),
            "inspect" => Some(NextActionKind::Inspect),
            "recompute" => Some(NextActionKind::Recompute),
            "recover" => Some(NextActionKind::Recover),
            _ => None,
        })
        .unwrap_or_else(|| infer_action_kind(&command, &surface));
    Some(TypedNextAction {
        action_id: action_id(&kind, &command, &surface),
        kind,
        command,
        expected_output: expected_output_from_projection(value),
        approval_required: approval_required_from_projection(value),
        artifact_refs: artifact_refs_from_projection(value),
        surface,
        reason,
    })
}

pub fn reduce_projection(value: &serde_json::Value) -> NextActionReducerOutput {
    let current = first_object(value, &["current_unit"])
        .or_else(|| first_object(value, &["primary_ready_task"]))
        .or_else(|| first_object(value, &["candidate_task_context", "ready_head"]))
        .or_else(|| first_object(value, &["selected_lanes"]));
    let current_unit = current.as_ref().and_then(|value| {
        let id = first_string(value, &[&["id"], &["task_id"]])?;
        Some(NextActionUnit {
            id,
            title: non_empty_string(value.get("title")),
            status: non_empty_string(value.get("status")),
        })
    });
    let lane_value = first_object(value, &["lane"])
        .or_else(|| first_object(value, &["selected_lanes"]))
        .or_else(|| {
            value
                .get("dispatch")
                .cloned()
                .filter(|value| value.is_object())
        });
    let lane = lane_value.as_ref().and_then(|value| {
        let id = first_string(
            value,
            &[&["id"], &["lane_id"], &["role_label"], &["dispatch_target"]],
        );
        let role = first_string(
            value,
            &[
                &["role"],
                &["runtime_role"],
                &["activation_runtime_role"],
                &["dispatch_target"],
            ],
        );
        let task_class = first_string(value, &[&["task_class"]]);
        let status = first_string(
            value,
            &[&["status"], &["lane_status"], &["dispatch_status"]],
        );
        (id.is_some() || role.is_some() || task_class.is_some() || status.is_some()).then_some(
            NextActionLane {
                id,
                role,
                task_class,
                status,
            },
        )
    });
    let packet = value.get("packet_materialization");
    let projection_cache = value.get("projection_cache").cloned();
    let packet_artifact = packet.and_then(|packet| first_object(packet, &["artifacts"]));
    let dispatch = value.get("dispatch");
    let packet_refs = NextActionReferences {
        run_id: first_string(
            value,
            &[
                &["dispatch", "run_id"],
                &["run_id"],
                &["current_unit", "id"],
            ],
        ),
        task_id: current_unit.as_ref().map(|unit| unit.id.clone()),
        packet_path: packet_artifact
            .as_ref()
            .and_then(|artifact| non_empty_string(artifact.get("dispatch_packet_path")))
            .or_else(|| {
                dispatch.and_then(|value| non_empty_string(value.get("dispatch_packet_path")))
            }),
        result_path: packet_artifact
            .as_ref()
            .and_then(|artifact| non_empty_string(artifact.get("dispatch_result_path")))
            .or_else(|| {
                dispatch.and_then(|value| non_empty_string(value.get("dispatch_result_path")))
            }),
        receipt_path: dispatch.and_then(|value| non_empty_string(value.get("receipt_path"))),
        source_refs: value
            .get("source_surfaces")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
    };
    let context_refs = NextActionReferences {
        run_id: packet_refs.run_id.clone(),
        task_id: packet_refs.task_id.clone(),
        packet_path: None,
        result_path: None,
        receipt_path: None,
        source_refs: [
            first_string(
                value,
                &[&["scope_task_id"], &["flow_projection", "flow_id"]],
            ),
            first_string(value, &[&["candidate_task_context", "admissibility_gate"]]),
            first_string(value, &[&["projection_source"], &["truth_source"]]),
        ]
        .into_iter()
        .flatten()
        .collect(),
    };
    let blocker_codes = value
        .get("blocker_codes")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .collect();
    let next_actions = value
        .get("next_actions")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .collect();
    reduce_next_action(NextActionReducerInput {
        status: non_empty_string(value.get("status")).unwrap_or_else(|| "blocked".to_string()),
        current_unit,
        lane,
        next_action: action_from_projection(value),
        projection_cache,
        packet_refs,
        context_refs,
        blocker_codes,
        next_actions,
        after_success: None,
    })
}

pub fn decorate_projection(mut value: serde_json::Value) -> serde_json::Value {
    let reduced = reduce_projection(&value);
    let reduced_value =
        serde_json::to_value(&reduced).expect("next action reducer should serialize");
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "next_action_reducer_version".to_string(),
            serde_json::json!(NEXT_ACTION_REDUCER_SCHEMA_VERSION),
        );
        object.insert("next_action_reducer".to_string(), reduced_value.clone());
        for key in [
            "current_unit",
            "lane",
            "next_action",
            "projection_cache",
            "packet_refs",
            "context_refs",
            "blockers",
            "after_success",
        ] {
            if let Some(field) = reduced_value.get(key) {
                object.insert(key.to_string(), field.clone());
            }
        }
    }
    value
}

pub fn cached_projection_admissible(cached: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(cached)
        .ok()
        .is_some_and(|value| {
            value
                .get("next_action_reducer_version")
                .and_then(serde_json::Value::as_str)
                == Some(NEXT_ACTION_REDUCER_SCHEMA_VERSION)
                && value
                    .get("next_action_reducer")
                    .is_some_and(serde_json::Value::is_object)
                && projection_cache_contract_admissible(&value)
        })
}

fn projection_cache_contract_admissible(value: &serde_json::Value) -> bool {
    let Some(cache) = value.get("projection_cache") else {
        return false;
    };
    projection_cache_control_contract_admissible(cache)
        || read_side_projection_cache_annotation_admissible(cache)
}

fn projection_cache_control_contract_admissible(cache: &serde_json::Value) -> bool {
    cache
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_some()
        && matches!(
            cache.get("mode").and_then(serde_json::Value::as_str),
            Some("auto" | "refresh" | "off")
        )
        && cache.get("hit").is_some_and(serde_json::Value::is_boolean)
        && cache
            .get("freshness")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|freshness| !freshness.trim().is_empty())
        && cache.get("recompute_reason").is_some()
}

fn read_side_projection_cache_annotation_admissible(cache: &serde_json::Value) -> bool {
    cache
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| !status.trim().is_empty())
        && cache
            .get("projection_name")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|projection_name| !projection_name.trim().is_empty())
        && cache
            .get("age_millis")
            .is_some_and(serde_json::Value::is_number)
        && cache
            .get("max_age_millis")
            .is_some_and(serde_json::Value::is_number)
        && cache
            .get("freshness_contract")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|freshness_contract| !freshness_contract.trim().is_empty())
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub fn consume_continue_command(run_id: Option<&str>) -> String {
    match run_id.map(str::trim).filter(|value| !value.is_empty()) {
        Some(run_id) => human_command(&format!(
            "vida taskflow consume continue --run-id {} --json",
            shell_quote(run_id)
        )),
        None => human_command("vida taskflow consume continue --json"),
    }
}

pub fn recovery_latest_command() -> String {
    human_command("vida taskflow recovery latest --json")
}

pub fn status_command() -> String {
    human_command("vida status --json")
}

pub fn human_recovery_status_command(run_id: &str) -> String {
    human_command(&format!(
        "vida taskflow recovery status {} --json",
        shell_quote(run_id)
    ))
}

pub fn human_lane_show_command(run_id: &str) -> String {
    human_command(&format!("vida lane show {} --json", shell_quote(run_id)))
}

pub fn human_run_graph_status_command(run_id: &str) -> String {
    human_command(&format!(
        "vida taskflow run-graph status {} --json",
        shell_quote(run_id)
    ))
}

pub fn human_task_next_lawful_command() -> String {
    human_command("vida task next-lawful")
}

pub fn human_taskflow_graph_summary_command() -> String {
    human_command("vida task validate-graph")
}

pub fn human_protocol_binding_repair_command() -> String {
    human_command("vida protocol binding repair")
}

pub fn human_closed_run_reconcile_command() -> String {
    human_command("vida task reconcile-closed-runs --limit 25 --json")
}

pub fn human_dependency_graph_repair_command() -> String {
    human_command("vida task validate-graph --json")
}

pub fn human_taskflow_protocol_binding_check_command() -> String {
    human_command("vida taskflow protocol-binding check --json")
}

pub fn human_taskflow_protocol_binding_sync_command() -> String {
    human_command("vida taskflow protocol-binding sync --json")
}

pub fn human_project_activator_command() -> String {
    human_command("vida project-activator --json")
}

pub fn human_bundle_check_command() -> String {
    human_command("vida taskflow consume bundle check --json")
}

pub fn human_lane_retire_command(run_id: &str) -> String {
    human_command(&format!(
        "vida lane retire {} --receipt-id <concrete-receipt-id> --reason <reason> --json",
        shell_quote(run_id)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_action_commands_are_human_readable() {
        assert_eq!(
            human_run_graph_status_command("run-1"),
            "vida taskflow run-graph status run-1"
        );
        assert_eq!(
            human_taskflow_graph_summary_command(),
            "vida task validate-graph"
        );
        assert_eq!(
            consume_continue_command(Some("run with space")),
            "vida taskflow consume continue --run-id 'run with space'"
        );
        assert_eq!(
            human_run_graph_status_command("run>pwned"),
            "vida taskflow run-graph status 'run>pwned'"
        );
        assert_eq!(
            human_recovery_status_command("run<secret"),
            "vida taskflow recovery status 'run<secret'"
        );
        assert_eq!(
            consume_continue_command(Some("run>pwned")),
            "vida taskflow consume continue --run-id 'run>pwned'"
        );
        assert_eq!(
            human_closed_run_reconcile_command(),
            "vida task reconcile-closed-runs --limit 25"
        );
    }

    #[test]
    fn decorated_projection_emits_complete_typed_next_action_contract() {
        let projection = serde_json::json!({
            "status": "blocked",
            "next_action": {
                "command": "vida task ready",
                "surface": "vida task ready",
                "reason": "inspect the authoritative ready projection"
            },
            "projection_cache": {
                "status": "hit",
                "mode": "auto",
                "hit": true,
                "freshness": "fresh",
                "recompute_reason": null
            },
            "expected_output": ["status", "blocker_codes"],
            "approval_required": true,
            "artifact_refs": {"surface": "vida taskflow next", "task_id": "task-1"},
            "blocker_codes": ["NoReadyTasks"],
            "next_actions": ["Run vida task ready"]
        });

        let decorated = decorate_projection(projection);
        let action = &decorated["next_action"];
        assert!(
            action["action_id"]
                .as_str()
                .is_some_and(|id| { id.starts_with("next_action_inspect_vida_task_ready") })
        );
        assert_eq!(action["kind"], "inspect");
        assert_eq!(action["command"], "vida task ready");
        assert_eq!(
            action["expected_output"],
            serde_json::json!(["status", "blocker_codes"])
        );
        assert_eq!(action["approval_required"], true);
        assert_eq!(action["artifact_refs"]["task_id"], "task-1");
        assert_eq!(action["surface"], "vida task ready");
        assert_eq!(decorated["projection_cache"]["mode"], "auto");
        assert_eq!(
            decorated["next_action_reducer"]["projection_cache"]["freshness"],
            "fresh"
        );
        assert_eq!(
            action["reason"],
            "inspect the authoritative ready projection"
        );
        assert!(cached_projection_admissible(
            &serde_json::to_string(&decorated).expect("decorated projection should serialize")
        ));
    }

    #[test]
    fn cached_projection_accepts_read_side_recent_projection_annotation() {
        let decorated = decorate_projection(serde_json::json!({
            "status": "pass",
            "next_action": {
                "command": "vida agent-dispatch next --json",
                "surface": "vida agent-dispatch next",
                "reason": "serve the cached dispatch projection"
            },
            "projection_cache": {
                "status": "recent_projection",
                "projection_name": "agent-dispatch-next-full-json",
                "age_millis": 17,
                "max_age_millis": 60000,
                "freshness_contract": "recent_bounded_stale_ok_for_read_only_operator_query"
            }
        }));

        assert!(cached_projection_admissible(
            &serde_json::to_string(&decorated).expect("decorated projection should serialize")
        ));
    }

    #[test]
    fn cached_projection_requires_reducer_schema() {
        assert!(!cached_projection_admissible(
            r#"{"status":"blocked","next_action":{"command":"vida task ready"}}"#
        ));
        let decorated = decorate_projection(serde_json::json!({
            "status": "pass",
            "next_action": {
                "command": "vida task show task-1",
                "surface": "vida task show",
                "reason": "inspect the ready task"
            },
            "projection_cache": {
                "status": "miss",
                "mode": "auto",
                "hit": false,
                "freshness": "recomputed",
                "recompute_reason": "cache_missing"
            }
        }));
        assert!(cached_projection_admissible(
            &serde_json::to_string(&decorated).expect("decorated projection should serialize")
        ));
    }

    #[test]
    fn reduce_next_action_normalizes_blockers_and_builds_default_after_success() {
        let reduced = reduce_next_action(NextActionReducerInput {
            status: "blocked".to_string(),
            current_unit: None,
            lane: None,
            next_action: None,
            projection_cache: None,
            packet_refs: NextActionReferences {
                packet_path: Some(" packet.json ".to_string()),
                ..NextActionReferences::default()
            },
            context_refs: NextActionReferences::default(),
            blocker_codes: vec![
                " Migration_Required ".to_string(),
                " ".to_string(),
                "Second_Blocker".to_string(),
            ],
            next_actions: vec![
                " Resolve migration ".to_string(),
                " ".to_string(),
                "Inspect second blocker".to_string(),
            ],
            after_success: None,
        });

        assert_eq!(
            reduced.blocker_codes,
            vec!["migration_required", "second_blocker"]
        );
        assert_eq!(
            reduced.next_actions,
            vec!["Resolve migration", "Inspect second blocker"]
        );
        assert_eq!(
            reduced.blockers,
            vec![
                NextActionBlocker {
                    code: "migration_required".to_string(),
                    next_action: Some("Resolve migration".to_string()),
                },
                NextActionBlocker {
                    code: "second_blocker".to_string(),
                    next_action: Some("Inspect second blocker".to_string()),
                },
            ]
        );
        assert_eq!(reduced.after_success.next_command, "vida task next");
        assert!(reduced.after_success.requires_receipt);
        assert!(reduced
            .after_success
            .invariant
            .contains("receipt-backed result"));
    }

    #[test]
    fn reduce_projection_preserves_authoritative_refs_and_action_contract() {
        let reduced = reduce_projection(&serde_json::json!({
            "status": "ready",
            "current_unit": {
                "id": "unit-id",
                "title": "Unit title",
                "status": "in_progress"
            },
            "lane": {
                "id": "lane-id",
                "role": "coder",
                "task_class": "implementation",
                "status": "active"
            },
            "dispatch": {
                "run_id": "dispatch-run",
                "dispatch_packet_path": "dispatch-packet",
                "dispatch_result_path": "dispatch-result",
                "receipt_path": "receipt-path"
            },
            "run_id": "root-run",
            "packet_materialization": {
                "artifacts": [{
                    "dispatch_packet_path": "packet-path",
                    "dispatch_result_path": "packet-result"
                }]
            },
            "scope_task_id": "scope-task",
            "flow_projection": {
                "flow_id": "flow-id",
                "current_step": {"approval_required": false}
            },
            "candidate_task_context": {"admissibility_gate": "admissible"},
            "projection_source": "projection-source",
            "truth_source": "truth-source",
            "source_surfaces": ["source-surface"],
            "blocker_codes": [" Blocked_Code "],
            "next_actions": [" Resolve blocker "],
            "next_action": {
                "command": "vida task bind",
                "kind": "bind",
                "surface": "vida task bind",
                "reason": "bind the selected unit",
                "expected_output": ["status", "task_id"],
                "approval_required": true
            },
            "projection_cache": {"status": "hit", "mode": "auto"},
            "artifact_refs": {"source": "authoritative"}
        }));

        assert_eq!(
            reduced.current_unit,
            Some(NextActionUnit {
                id: "unit-id".to_string(),
                title: Some("Unit title".to_string()),
                status: Some("in_progress".to_string()),
            })
        );
        assert_eq!(
            reduced.lane,
            Some(NextActionLane {
                id: Some("lane-id".to_string()),
                role: Some("coder".to_string()),
                task_class: Some("implementation".to_string()),
                status: Some("active".to_string()),
            })
        );
        assert_eq!(reduced.packet_refs.run_id.as_deref(), Some("dispatch-run"));
        assert_eq!(reduced.packet_refs.task_id.as_deref(), Some("unit-id"));
        assert_eq!(reduced.packet_refs.packet_path.as_deref(), Some("packet-path"));
        assert_eq!(reduced.packet_refs.result_path.as_deref(), Some("packet-result"));
        assert_eq!(reduced.packet_refs.receipt_path.as_deref(), Some("receipt-path"));
        assert_eq!(reduced.packet_refs.source_refs, vec!["source-surface"]);
        assert_eq!(
            reduced.context_refs.source_refs,
            vec!["scope-task", "admissible", "projection-source"]
        );
        assert_eq!(reduced.blocker_codes, vec!["blocked_code"]);
        assert_eq!(reduced.next_actions, vec!["Resolve blocker"]);
        let action = reduced.next_action.expect("action should be present");
        assert_eq!(action.kind, NextActionKind::Bind);
        assert_eq!(action.command, "vida task bind");
        assert_eq!(action.expected_output, vec!["status", "task_id"]);
        assert!(action.approval_required);
        assert_eq!(action.artifact_refs, serde_json::json!({"source": "authoritative"}));
        assert_eq!(action.reason, "bind the selected unit");
        assert!(reduced.after_success.requires_receipt);
    }

    #[test]
    fn reduce_projection_uses_fallback_sources_and_infers_action_kinds() {
        let current_cases = [
            (
                serde_json::json!({"primary_ready_task": {"task_id": "primary"}}),
                "primary",
            ),
            (
                serde_json::json!({
                    "candidate_task_context": {"ready_head": [{"id": "candidate"}]}
                }),
                "candidate",
            ),
            (
                serde_json::json!({"selected_lanes": [{"id": "selected"}]}),
                "selected",
            ),
        ];
        for (projection, expected_id) in current_cases {
            assert_eq!(
                reduce_projection(&projection)
                    .current_unit
                    .expect("fallback current unit should be selected")
                    .id,
                expected_id
            );
        }

        let selected_lane = reduce_projection(&serde_json::json!({
            "selected_lanes": [{
                "lane_id": "selected-lane",
                "runtime_role": "worker",
                "task_class": "verification",
                "lane_status": "ready"
            }]
        }))
        .lane
        .expect("selected lane should be selected");
        assert_eq!(selected_lane.id.as_deref(), Some("selected-lane"));
        assert_eq!(selected_lane.role.as_deref(), Some("worker"));
        assert_eq!(selected_lane.task_class.as_deref(), Some("verification"));
        assert_eq!(selected_lane.status.as_deref(), Some("ready"));

        let dispatch_lane = reduce_projection(&serde_json::json!({
            "dispatch": {"dispatch_target": "dispatch-lane", "dispatch_status": "queued"}
        }))
        .lane
        .expect("dispatch lane should be selected");
        assert_eq!(dispatch_lane.id.as_deref(), Some("dispatch-lane"));
        assert_eq!(dispatch_lane.role.as_deref(), Some("dispatch-lane"));
        assert_eq!(dispatch_lane.status.as_deref(), Some("queued"));

        let action_cases = [
            ("vida task bind", NextActionKind::Bind),
            ("vida taskflow continue", NextActionKind::Continue),
            ("vida task recover", NextActionKind::Recover),
            ("vida task refresh", NextActionKind::Recompute),
            ("vida agent dispatch", NextActionKind::Execute),
            ("vida task inspect", NextActionKind::Inspect),
        ];
        for (command, expected_kind) in action_cases {
            let reduced = reduce_projection(&serde_json::json!({
                "next_action": {"command": command}
            }));
            assert_eq!(
                reduced.next_action.expect("command should produce action").kind,
                expected_kind
            );
        }
    }

    #[test]
    fn decorate_projection_overwrites_stale_reduced_fields() {
        let decorated = decorate_projection(serde_json::json!({
            "status": "blocked",
            "current_unit": "stale",
            "lane": "stale",
            "next_action": {"command": "vida task continue", "surface": "continue"},
            "projection_cache": "stale",
            "packet_refs": "stale",
            "context_refs": "stale",
            "blockers": "stale",
            "after_success": "stale",
            "projection_cache_source": {"status": "hit"}
        }));

        assert_eq!(
            decorated["next_action_reducer_version"],
            NEXT_ACTION_REDUCER_SCHEMA_VERSION
        );
        for key in [
            "current_unit",
            "lane",
            "next_action",
            "projection_cache",
            "packet_refs",
            "context_refs",
            "blockers",
            "after_success",
        ] {
            assert_eq!(
                decorated[key], decorated["next_action_reducer"][key],
                "decorated field {key} should mirror reducer output"
            );
        }
    }

    #[test]
    fn first_object_handles_arrays_objects_and_empty_values() {
        assert_eq!(
            first_object(&serde_json::json!({"rows": [{"id": "first"}]}), &["rows"]),
            Some(serde_json::json!({"id": "first"}))
        );
        assert_eq!(
            first_object(&serde_json::json!({"row": {"id": "object"}}), &["row"]),
            Some(serde_json::json!({"id": "object"}))
        );
        assert_eq!(
            first_object(&serde_json::json!({"rows": []}), &["rows"]),
            None
        );
        assert_eq!(
            first_object(&serde_json::json!({"rows": "not-an-object"}), &["rows"]),
            None
        );
    }
}
