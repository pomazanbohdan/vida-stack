use std::process::ExitCode;
use std::time::Duration;

use crate::{
    state_store::StateStore, DiagnosticsArgs, DiagnosticsCommand, DiagnosticsPostCommitArgs,
};

const DIAGNOSTICS_LOCK_TIMEOUT: Duration = Duration::from_secs(15);
const POST_COMMIT_DIAGNOSTICS_PROJECTION_NAME: &str = "diagnostics-post-commit-latest";

fn command_output(program: &str, args: &[&str]) -> serde_json::Value {
    match std::process::Command::new(program).args(args).output() {
        Ok(output) => serde_json::json!({
            "status": if output.status.success() { "pass" } else { "blocked" },
            "exit_code": output.status.code(),
            "stdout": String::from_utf8_lossy(&output.stdout).trim(),
            "stderr": String::from_utf8_lossy(&output.stderr).trim(),
        }),
        Err(error) => serde_json::json!({
            "status": "blocked",
            "exit_code": null,
            "stdout": "",
            "stderr": error.to_string(),
        }),
    }
}

fn git_status_summary() -> serde_json::Value {
    let status = command_output("git", &["status", "--short", "--branch"]);
    let stdout = status["stdout"].as_str().unwrap_or_default();
    let dirty = stdout.lines().skip(1).any(|line| !line.trim().is_empty());
    serde_json::json!({
        "status": if status["status"] == "pass" && !dirty { "pass" } else { "blocked" },
        "dirty": dirty,
        "branch_summary": stdout.lines().next().unwrap_or_default(),
        "raw": status,
    })
}

fn recovery_projected_task_id(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> Option<String> {
    recovery
        .and_then(|summary| (!summary.task_id.trim().is_empty()).then(|| summary.task_id.clone()))
}

fn recovery_is_terminal_retired_runtime_run(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> bool {
    recovery.is_some_and(|summary| {
        summary.resume_status == "completed"
            && summary.lifecycle_stage == "closure_complete"
            && !summary.delegation_gate.delegated_cycle_open
            && summary.delegation_gate.blocker_code.is_none()
            && summary.resume_target == "none"
            && summary.task_id == summary.run_id
    })
}

fn binding_projected_task_id(
    binding: Option<&crate::state_store::RunGraphContinuationBinding>,
) -> Option<String> {
    let binding = binding?;
    if binding.status != "bound" {
        return None;
    }
    binding
        .active_bounded_unit
        .get("task_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            let task_id = binding.task_id.trim();
            (!task_id.is_empty()).then(|| task_id.to_string())
        })
}

fn missing_task_actionability(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    binding: Option<&crate::state_store::RunGraphContinuationBinding>,
    task_ids: &[String],
) -> serde_json::Value {
    let projected = binding_projected_task_id(binding)
        .map(|task_id| {
            (
                task_id,
                binding.map(|binding| binding.run_id.as_str()),
                "explicit_continuation_binding",
            )
        })
        .or_else(|| {
            recovery_projected_task_id(recovery).map(|task_id| {
                (
                    task_id,
                    recovery.map(|summary| summary.run_id.as_str()),
                    "run_graph_recovery",
                )
            })
        });
    let Some((task_id, run_id, source)) = projected else {
        return serde_json::json!({
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "checked_task_id": null,
            "checked_source": null,
        });
    };
    if source == "run_graph_recovery" && recovery_is_terminal_retired_runtime_run(recovery) {
        return serde_json::json!({
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "checked_task_id": task_id,
            "checked_source": source,
            "terminal_runtime_run_without_task": true,
        });
    }
    if task_ids.iter().any(|id| id == &task_id) {
        return serde_json::json!({
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "checked_task_id": task_id,
            "checked_source": source,
        });
    }
    serde_json::json!({
        "status": "blocked",
        "blocker_codes": ["next_action_target_missing"],
        "next_actions": [
            crate::status_surface_signals::runtime_binding_task_missing_next_action(
                run_id,
                &task_id,
            ),
            "Inspect `vida orchestrator-session show --json` and reconcile stale session ownership before binding continuation."
        ],
        "checked_task_id": task_id,
        "checked_source": source,
    })
}

fn status_from_blockers(blockers: &[String]) -> &'static str {
    if blockers.is_empty() {
        "pass"
    } else {
        "blocked"
    }
}

fn diagnostic_exit_code(payload: &serde_json::Value) -> ExitCode {
    if payload["status"] == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn compact_counted_json_member(value: &serde_json::Value) -> serde_json::Value {
    let count = value
        .as_array()
        .map(|rows| rows.len())
        .or_else(|| value.as_object().map(|rows| rows.len()))
        .unwrap_or(0);
    serde_json::json!({
        "count": count,
        "detail": "omitted_from_fast_post_commit_diagnostics"
    })
}

fn compact_host_dispatch_preflight_for_diagnostics(
    payload: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "host_cli_system": payload["host_cli_system"].clone(),
        "runtime_surface": payload["runtime_surface"].clone(),
        "runtime_root": payload["runtime_root"].clone(),
        "effective_execution_posture": payload["effective_execution_posture"].clone(),
        "mixed_posture": payload["mixed_posture"].clone(),
        "hybrid_external_cli_relevant": payload["hybrid_external_cli_relevant"].clone(),
        "agents": compact_counted_json_member(&payload["agents"]),
        "subagent_backends": compact_counted_json_member(&payload["subagent_backends"]),
        "internal_dispatch_alias_count": payload["internal_dispatch_alias_count"].clone(),
        "internal_dispatch_alias_load_error": payload["internal_dispatch_alias_load_error"].clone(),
        "external_cli_preflight": {
            "status": payload["external_cli_preflight"]["status"].clone(),
            "selected_execution_class": payload["external_cli_preflight"]["selected_execution_class"].clone(),
            "effective_execution_posture": payload["external_cli_preflight"]["effective_execution_posture"].clone(),
            "requires_external_cli": payload["external_cli_preflight"]["requires_external_cli"].clone(),
            "hybrid_external_cli_relevant": payload["external_cli_preflight"]["hybrid_external_cli_relevant"].clone(),
            "blocked_primary_backends": payload["external_cli_preflight"]["blocked_primary_backends"].clone(),
            "blocked_required_primary_backends": payload["external_cli_preflight"]["blocked_required_primary_backends"].clone(),
            "blocker_code": payload["external_cli_preflight"]["blocker_code"].clone(),
        },
    })
}

async fn build_post_commit_diagnostics(
    state_dir: std::path::PathBuf,
) -> Result<serde_json::Value, String> {
    let git_status = git_status_summary();
    let owner_evidence =
        crate::orchestrator_session_surface::compact_runtime_owner_evidence_for_operator(
            crate::orchestrator_session_surface::build_runtime_owner_evidence(&state_dir, true)?,
        );
    let store = StateStore::open_existing_read_only_with_timeout(
        state_dir.clone(),
        DIAGNOSTICS_LOCK_TIMEOUT,
    )
    .await
    .map_err(|error| format!("open state store for diagnostics: {error}"))?;

    let latest_run_graph_status = store
        .latest_run_graph_status_for_current_session()
        .await
        .map_err(|error| format!("read latest run graph status: {error}"))?;
    let latest_run_graph_recovery = store
        .latest_run_graph_recovery_summary_for_current_session()
        .await
        .map_err(|error| format!("read latest run graph recovery: {error}"))?;
    let latest_dispatch_receipt = store
        .latest_run_graph_dispatch_receipt_summary_for_current_session()
        .await
        .map_err(|error| format!("read latest run graph dispatch receipt: {error}"))?;
    let task_ids = store
        .all_tasks()
        .await
        .map_err(|error| format!("read TaskFlow tasks: {error}"))?
        .into_iter()
        .map(|task| task.id)
        .collect::<Vec<_>>();
    let latest_explicit_binding = store
        .latest_explicit_run_graph_continuation_binding_for_current_session()
        .await
        .map_err(|error| format!("read latest explicit continuation binding: {error}"))?;
    let target_actionability = missing_task_actionability(
        latest_run_graph_recovery.as_ref(),
        latest_explicit_binding.as_ref(),
        &task_ids,
    );

    let runtime_consumption = crate::runtime_consumption_summary(store.root())
        .map_err(|error| format!("read runtime-consumption summary: {error}"))?;
    let host_dispatch_preflight =
        crate::status_surface_host_agents::build_host_agent_status_summary(
            std::env::current_dir()
                .map_err(|error| format!("resolve current dir: {error}"))?
                .as_path(),
        )
        .unwrap_or_else(|| {
            serde_json::json!({
                "status": "blocked",
                "blocker_code": "host_dispatch_preflight_missing",
                "next_actions": ["Run `vida status --json` from a VIDA-initialized project root."]
            })
        });
    let host_dispatch_preflight =
        compact_host_dispatch_preflight_for_diagnostics(host_dispatch_preflight);

    let mut blocker_codes = Vec::<String>::new();
    if git_status["status"] != "pass" {
        blocker_codes.push("git_status_blocked".to_string());
    }
    if owner_evidence["mutation_gate"] == "blocked_live_other_orchestrator" {
        blocker_codes.push("live_other_orchestrator_owner".to_string());
    }
    if target_actionability["status"] == "blocked" {
        blocker_codes.push("next_action_target_missing".to_string());
    }
    if latest_run_graph_status
        .as_ref()
        .is_some_and(|status| status.status == "blocked")
    {
        blocker_codes.push("latest_run_graph_status_blocked".to_string());
    }
    blocker_codes.sort();
    blocker_codes.dedup();

    let status = status_from_blockers(&blocker_codes);
    let recommended_issue_workflow = serde_json::json!({
        "upstream_issue_owner": crate::orchestrator_session_surface::issue_owner(),
        "open_issue_search_terms": [
            "continuation_binding_ambiguous",
            "next_action_target_missing",
            "post-commit runtime diagnostics",
            "orchestrator session ownership"
        ],
        "duplicate_policy": "search_open_issues_first_comment_matching_issue_do_not_create_duplicate",
        "project_local_clean_completion": git_status["status"] == "pass",
        "upstream_runtime_defect": status == "blocked",
    });

    Ok(serde_json::json!({
        "surface": "vida diagnostics post-commit",
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": if status == "pass" {
            Vec::<String>::new()
        } else {
            vec![
                "Inspect the blocked diagnostic sections before reporting closure.".to_string(),
                "If this is a VIDA runtime defect, search/comment/create only in the upstream VIDA stack issue tracker.".to_string()
            ]
        },
        "git_status": git_status,
        "taskflow_status": {
            "task_count": task_ids.len(),
            "latest_run_graph_status": latest_run_graph_status,
            "latest_run_graph_recovery": latest_run_graph_recovery,
            "latest_dispatch_receipt": latest_dispatch_receipt,
            "latest_explicit_continuation_binding": latest_explicit_binding,
            "continuation_target_actionability": target_actionability,
        },
        "docflow_status": {
            "status": "not_executed_by_diagnostic_surface",
            "canonical_checks": [
                "vida docflow proofcheck --profile active-canon",
                "vida docflow check --root . <changed-doc>"
            ],
            "reason": "post-commit diagnostics reports DocFlow command contract without mutating or recursively invoking DocFlow",
        },
        "runtime_consumption": runtime_consumption,
        "canonical_continuation_run_id": latest_run_graph_status
            .as_ref()
            .map(|status| status.run_id.clone())
            .or_else(|| latest_run_graph_recovery.as_ref().map(|recovery| recovery.run_id.clone())),
        "open_delegated_cycle_state": latest_run_graph_status
            .as_ref()
            .map(|status| status.delegation_gate()),
        "host_dispatch_preflight": host_dispatch_preflight,
        "model_cli_compatibility_status": {
            "status": "reported_by_host_dispatch_preflight",
            "source": "host_dispatch_preflight"
        },
        "runtime_owner_evidence": owner_evidence,
        "downstream_execution_context": crate::orchestrator_session_surface::context_summary_map(&state_dir),
        "upstream_vida_publication_context": {
            "issue_owner": crate::orchestrator_session_surface::issue_owner(),
            "issue_tracker_url": format!(
                "https://github.com/{}/issues",
                crate::orchestrator_session_surface::issue_owner()
            ),
        },
        "recommended_issue_workflow": recommended_issue_workflow,
    }))
}

async fn run_post_commit(args: DiagnosticsPostCommitArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(crate::state_store::default_state_dir);
    match build_post_commit_diagnostics(state_dir.clone()).await {
        Ok(payload) => {
            if args.json {
                crate::print_json_pretty(&payload);
                crate::operator_projection_cache::write_json_projection(
                    &state_dir,
                    POST_COMMIT_DIAGNOSTICS_PROJECTION_NAME,
                    &payload,
                );
            } else {
                println!("VIDA post-commit diagnostics");
                println!(
                    "status: {}",
                    payload["status"].as_str().unwrap_or("blocked")
                );
                if let Some(blockers) = payload["blocker_codes"].as_array() {
                    println!("blocker_codes: {}", blockers.len());
                }
            }
            diagnostic_exit_code(&payload)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

pub(crate) async fn run_diagnostics(args: DiagnosticsArgs) -> ExitCode {
    match args.command {
        DiagnosticsCommand::PostCommit(args) => run_post_commit(args).await,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compact_host_dispatch_preflight_for_diagnostics, missing_task_actionability,
        run_post_commit, POST_COMMIT_DIAGNOSTICS_PROJECTION_NAME,
    };
    use crate::test_cli_support::guard_current_dir;
    use crate::DiagnosticsPostCommitArgs;
    use std::fs;
    use std::process::ExitCode;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn diagnostics_post_commit_compacts_heavy_operator_evidence() {
        let payload = serde_json::json!({
            "agents": {
                "analyst": {"model": "gpt-5.5"},
                "coach": {"model": "gpt-5.5"}
            },
            "subagent_backends": [
                {"backend": "codex"},
                {"backend": "opencode"},
                {"backend": "hermes"}
            ],
            "host_cli_system": "codex"
        });

        let compacted = compact_host_dispatch_preflight_for_diagnostics(payload);

        assert_eq!(compacted["agents"]["count"], 2);
        assert_eq!(compacted["subagent_backends"]["count"], 3);
        assert_eq!(compacted["host_cli_system"], "codex");
    }

    #[tokio::test]
    async fn diagnostics_post_commit_rejects_state_marker_stale_cached_projection() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-diagnostics-post-commit-stale-cache-{}-{}",
            std::process::id(),
            nanos
        ));
        let state_dir = root.join(".vida").join("data").join("state");
        crate::state_store::StateStore::open(state_dir.clone())
            .await
            .expect("state store should initialize");
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&root)
            .output()
            .expect("git init should run");
        fs::write(root.join(".git").join("info").join("exclude"), ".vida/\n")
            .expect("git exclude should be writable");

        crate::operator_projection_cache::write_json_projection(
            &state_dir,
            POST_COMMIT_DIAGNOSTICS_PROJECTION_NAME,
            &serde_json::json!({
                "surface": "vida diagnostics post-commit",
                "status": "pass",
                "blocker_codes": [],
                "recommended_issue_workflow": {
                    "project_local_clean_completion": true,
                    "upstream_runtime_defect": false
                }
            }),
        );
        std::thread::sleep(Duration::from_millis(10));
        crate::operator_projection_cache::touch_state_mutation_marker(&state_dir);

        let _cwd = guard_current_dir(&root);
        let code = run_post_commit(DiagnosticsPostCommitArgs {
            state_dir: Some(state_dir.clone()),
            json: true,
        })
        .await;

        assert_eq!(code, ExitCode::SUCCESS);
        let rewritten = fs::read_to_string(
            state_dir
                .join("operator-projections")
                .join(format!("{POST_COMMIT_DIAGNOSTICS_PROJECTION_NAME}.json")),
        )
        .expect("diagnostics projection should be rewritten");
        let rewritten: serde_json::Value =
            serde_json::from_str(&rewritten).expect("rewritten projection should be json");
        assert_ne!(rewritten["cached"], true);
        assert!(rewritten.get("projection_cache").is_none());
        assert!(rewritten["taskflow_status"].is_object());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn diagnostics_blocks_continuation_bind_to_missing_task() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-1".to_string(),
            task_id: "missing-task".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_blocked".to_string(),
            handoff_state: "none".to_string(),
            checkpoint_kind: "none".to_string(),
            policy_gate: "tool_execution_failed".to_string(),
            resume_status: "blocked".to_string(),
            resume_target: "none".to_string(),
            resume_node: None,
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                lifecycle_stage: "analysis_blocked".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_blocked".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                blocker_code: Some("tool_execution_failed".to_string()),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
            },
        };
        let payload =
            missing_task_actionability(Some(&recovery), None, &["other-task".to_string()]);
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["blocker_codes"][0], "next_action_target_missing");
        assert!(payload["next_actions"][0].as_str().is_some_and(|action| {
            action.contains("missing-task")
                && action.contains("vida taskflow recovery status run-1 --json")
                && action.contains("closure_complete")
                && !action.contains("vida taskflow continuation bind")
        }));
    }

    #[test]
    fn diagnostics_actionability_prefers_explicit_bound_task_over_terminal_recovery_task() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "closed-run".to_string(),
            task_id: "closed-run".to_string(),
            active_node: "closure".to_string(),
            lifecycle_stage: "closure_complete".to_string(),
            handoff_state: "none".to_string(),
            checkpoint_kind: "none".to_string(),
            policy_gate: "not_required".to_string(),
            resume_status: "completed".to_string(),
            resume_target: "none".to_string(),
            resume_node: None,
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "closure".to_string(),
                lifecycle_stage: "closure_complete".to_string(),
                delegated_cycle_open: false,
                delegated_cycle_state: "clear".to_string(),
                local_exception_takeover_gate: "delegated_cycle_clear".to_string(),
                blocker_code: None,
                reporting_pause_gate: "closure_candidate".to_string(),
                continuation_signal: "continue_after_reports".to_string(),
            },
        };
        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "closed-run".to_string(),
            task_id: "bound-task".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "task_graph_task",
                "run_id": "closed-run",
                "task_id": "bound-task",
                "task_status": "open"
            }),
            binding_source: "explicit_continuation_bind_task".to_string(),
            why_this_unit: "test explicit binding".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
            request_text: Some("Fix bounded task".to_string()),
            recorded_at: "2026-05-13T09:04:25Z".to_string(),
        };

        let payload = missing_task_actionability(
            Some(&recovery),
            Some(&binding),
            &["closed-run".to_string(), "bound-task".to_string()],
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["checked_task_id"], "bound-task");
        assert_eq!(payload["checked_source"], "explicit_continuation_binding");
    }

    #[test]
    fn diagnostics_allows_terminal_retired_runtime_run_without_task_row() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "runtime-vida-taskflow-codex".to_string(),
            task_id: "runtime-vida-taskflow-codex".to_string(),
            active_node: "closure".to_string(),
            lifecycle_stage: "closure_complete".to_string(),
            handoff_state: "none".to_string(),
            checkpoint_kind: "none".to_string(),
            policy_gate: "closed_task_stale_run_retired".to_string(),
            resume_status: "completed".to_string(),
            resume_target: "none".to_string(),
            resume_node: None,
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "closure".to_string(),
                lifecycle_stage: "closure_complete".to_string(),
                delegated_cycle_open: false,
                delegated_cycle_state: "clear".to_string(),
                local_exception_takeover_gate: "delegated_cycle_clear".to_string(),
                blocker_code: None,
                reporting_pause_gate: "closure_candidate".to_string(),
                continuation_signal: "continue_after_reports".to_string(),
            },
        };

        let payload = missing_task_actionability(Some(&recovery), None, &[]);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["checked_task_id"], "runtime-vida-taskflow-codex");
        assert_eq!(payload["checked_source"], "run_graph_recovery");
        assert_eq!(payload["terminal_runtime_run_without_task"], true);
    }
}
