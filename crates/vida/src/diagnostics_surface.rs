use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use crate::{
    state_store::StateStore, DiagnosticsArgs, DiagnosticsCommand, DiagnosticsEvidenceCheckArgs,
    DiagnosticsPostCommitArgs, DiagnosticsRulesCheckArgs,
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

fn recovery_summary_is_completed_terminal_closure_for_task(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    run_id: Option<&str>,
    task_id: &str,
) -> bool {
    let Some(summary) = recovery else {
        return false;
    };
    let Some(run_id) = run_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    summary.run_id == run_id && summary.task_id == task_id && summary.is_terminal_closure()
}

fn missing_task_actionability(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    binding: Option<&crate::state_store::RunGraphContinuationBinding>,
    task_ids: &[String],
    closed_task_ids: &[String],
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
    if source == "run_graph_recovery"
        && crate::runtime_dispatch_receipt_helpers::recovery_summary_is_terminal_retired_runtime_run(
            recovery,
        )
    {
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
        if closed_task_ids.iter().any(|id| id == &task_id) {
            if recovery_summary_is_completed_terminal_closure_for_task(recovery, run_id, &task_id) {
                return serde_json::json!({
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "checked_task_id": task_id,
                    "checked_source": source,
                    "task_status": "closed",
                    "terminal_closure_recovery": true,
                });
            }
            return serde_json::json!({
                "status": "blocked",
                "blocker_codes": ["closed_task_active_run_projection_mismatch"],
                "next_actions": [
                    closed_task_active_run_projection_mismatch_next_action(),
                    crate::status_surface_signals::runtime_binding_task_missing_next_action(
                        run_id,
                        &task_id,
                    ),
                ],
                "checked_task_id": task_id,
                "checked_source": source,
                "task_status": "closed",
            });
        }
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

fn closed_task_active_run_projection_mismatch_next_action() -> String {
    "Run `vida task reconcile-closed-runs --limit 25` and inspect skipped runs with `vida taskflow run-graph status <run-id>`; closed tasks must not remain projected as active runtime work."
        .to_string()
}

fn post_commit_default_clear_command(payload: &serde_json::Value) -> Option<&'static str> {
    let blocked_by_closed_task_projection =
        payload["blocker_codes"].as_array().is_some_and(|blockers| {
            blockers
                .iter()
                .any(|code| code.as_str() == Some("closed_task_active_run_projection_mismatch"))
        });
    blocked_by_closed_task_projection.then_some("vida task reconcile-closed-runs --limit 25")
}

fn post_commit_closed_task_active_run_projection_mismatch(
    latest_run_graph_status: Option<&crate::state_store::RunGraphStatus>,
    latest_terminal_task_active_run_graph_task_stale: bool,
    closed_task_ids: &[String],
    latest_run_graph_terminal_closure_has_truth: bool,
) -> bool {
    let latest_run_graph_task_closed = latest_run_graph_status.is_some_and(|status| {
        closed_task_ids.iter().any(|id| id == &status.task_id)
            && !crate::taskflow_run_graph_task_authority::run_graph_status_is_terminal_closure(
                status,
            )
    });
    (!latest_run_graph_terminal_closure_has_truth && latest_run_graph_task_closed)
        || latest_terminal_task_active_run_graph_task_stale
}

fn diagnostic_exit_code(payload: &serde_json::Value) -> ExitCode {
    if payload["status"] == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn trimmed_non_empty_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn normalized_path_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .filter(|value| !value.trim().is_empty())
        .collect()
}

fn render_diagnostics_gate_payload(
    surface: &str,
    gate_id: &str,
    gate_status: &str,
    task_id: Option<&str>,
    evidence_refs: Vec<String>,
    affected_paths: Vec<String>,
    blocker_codes: Vec<String>,
    failure_codes: Vec<String>,
    issues: Vec<serde_json::Value>,
    next_actions: Vec<String>,
) -> serde_json::Value {
    let status = status_from_blockers(&blocker_codes);
    let artifact_refs = serde_json::json!({
        "surface": surface,
        "task_id": task_id.unwrap_or_default(),
        "evidence_refs": evidence_refs,
        "affected_paths": affected_paths,
    });
    let vida_gate_result = crate::release1_operator_output::render_vida_gate_result_with_status(
        gate_id,
        gate_status,
        blocker_codes.clone(),
        Vec::new(),
        failure_codes,
        issues,
        next_actions.clone(),
        artifact_refs.clone(),
    );
    serde_json::json!({
        "surface": surface,
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": if status == "pass" { Vec::<String>::new() } else { next_actions },
        "artifact_refs": artifact_refs,
        "operator_contracts": vida_gate_result["operator_contracts"].clone(),
        "vida_gate_result": vida_gate_result,
    })
}

fn build_evidence_check_diagnostics(args: &DiagnosticsEvidenceCheckArgs) -> serde_json::Value {
    let evidence_refs = trimmed_non_empty_values(&args.evidence_refs);
    let mut blocker_codes = Vec::new();
    let mut issues = Vec::new();
    let mut next_actions = Vec::new();
    if evidence_refs.is_empty() {
        blocker_codes.push("missing_gate_evidence".to_string());
        issues.push(serde_json::json!({
            "code": "insufficient_evidence",
            "message": "No evidence refs were supplied for the bounded gate.",
        }));
        next_actions.push(
            "Provide at least one concrete --evidence-ref value before treating the gate as pass."
                .to_string(),
        );
    }
    render_diagnostics_gate_payload(
        "vida diagnostics evidence-check",
        "diagnostics.evidence_check",
        if blocker_codes.is_empty() {
            "pass"
        } else {
            "insufficient_evidence"
        },
        args.task_id.as_deref(),
        evidence_refs,
        Vec::new(),
        blocker_codes,
        Vec::new(),
        issues,
        next_actions,
    )
}

fn check_protocol_id(protocol_id: &str) -> Option<serde_json::Value> {
    crate::protocol_surface::render_protocol_view_target(protocol_id)
        .err()
        .map(|error| {
            serde_json::json!({
                "code": "protocol_rule_violation",
                "protocol_id": protocol_id,
                "message": error,
            })
        })
}

fn check_changed_path(path: &Path) -> Option<serde_json::Value> {
    if path.exists() {
        None
    } else {
        Some(serde_json::json!({
            "code": "rules_check_path_missing",
            "path": path.display().to_string(),
            "message": "Changed path does not exist in the current project root.",
        }))
    }
}

fn build_rules_check_diagnostics(args: &DiagnosticsRulesCheckArgs) -> serde_json::Value {
    let protocol_ids = trimmed_non_empty_values(&args.protocol_ids);
    let affected_paths = normalized_path_strings(&args.changed_paths);
    let evidence_refs = protocol_ids
        .iter()
        .map(|protocol_id| format!("protocol:{protocol_id}"))
        .chain(affected_paths.iter().map(|path| format!("path:{path}")))
        .collect::<Vec<_>>();
    let mut blocker_codes = Vec::new();
    let mut issues = Vec::new();
    let mut next_actions = Vec::new();

    if protocol_ids.is_empty() && affected_paths.is_empty() {
        blocker_codes.push("missing_gate_evidence".to_string());
        issues.push(serde_json::json!({
            "code": "insufficient_evidence",
            "message": "Rules-check needs at least one --changed-path or --protocol-id input.",
        }));
        next_actions.push(
            "Run rules-check with concrete --changed-path or --protocol-id inputs.".to_string(),
        );
    }

    for protocol_id in &protocol_ids {
        if let Some(issue) = check_protocol_id(protocol_id) {
            blocker_codes.push("protocol_rule_violation".to_string());
            issues.push(issue);
        }
    }
    for path in &args.changed_paths {
        if let Some(issue) = check_changed_path(path) {
            blocker_codes.push("rules_check_path_missing".to_string());
            issues.push(issue);
        }
    }
    blocker_codes.sort();
    blocker_codes.dedup();
    if !issues.is_empty() && next_actions.is_empty() {
        next_actions.push(
            "Resolve rules-check issues before treating the bounded change as pass.".to_string(),
        );
    }

    render_diagnostics_gate_payload(
        "vida diagnostics rules-check",
        "diagnostics.rules_check",
        if blocker_codes
            .iter()
            .any(|code| code == "missing_gate_evidence")
        {
            "insufficient_evidence"
        } else if blocker_codes.is_empty() {
            "pass"
        } else {
            "blocked"
        },
        args.task_id.as_deref(),
        evidence_refs,
        affected_paths,
        blocker_codes,
        Vec::new(),
        issues,
        next_actions,
    )
}

fn run_diagnostics_gate(payload: serde_json::Value, json: bool) -> ExitCode {
    if json {
        crate::print_json_pretty(&payload);
    } else {
        println!(
            "{}",
            payload["surface"].as_str().unwrap_or("vida diagnostics")
        );
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
    let latest_terminal_task_active_run_graph_status = store
        .latest_terminal_task_active_run_graph_status()
        .await
        .map_err(|error| format!("read latest terminal-task run graph status: {error}"))?;
    let latest_run_graph_recovery = store
        .latest_run_graph_recovery_summary_for_current_session()
        .await
        .map_err(|error| format!("read latest run graph recovery: {error}"))?;
    let latest_dispatch_receipt = store
        .latest_run_graph_dispatch_receipt_summary_for_current_session()
        .await
        .map_err(|error| format!("read latest run graph dispatch receipt: {error}"))?;
    let tasks = store
        .all_tasks()
        .await
        .map_err(|error| format!("read TaskFlow tasks: {error}"))?;
    let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
    let closed_task_ids = tasks
        .iter()
        .filter(|task| crate::state_store::StateStore::task_status_is_closed_like(&task.status))
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    let latest_explicit_binding = store
        .latest_explicit_run_graph_continuation_binding_for_current_session()
        .await
        .map_err(|error| format!("read latest explicit continuation binding: {error}"))?;
    let explicit_binding_run_graph_recovery = match latest_explicit_binding.as_ref() {
        Some(binding) => store
            .run_graph_status(&binding.run_id)
            .await
            .ok()
            .map(crate::state_store::RunGraphRecoverySummary::from_status),
        None => None,
    };
    let target_actionability_recovery = explicit_binding_run_graph_recovery
        .as_ref()
        .or(latest_run_graph_recovery.as_ref());
    let target_actionability = missing_task_actionability(
        target_actionability_recovery,
        latest_explicit_binding.as_ref(),
        &task_ids,
        &closed_task_ids,
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
                "next_actions": ["Run `vida status` from a VIDA-initialized project root."]
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
        if let Some(codes) = target_actionability["blocker_codes"].as_array() {
            blocker_codes.extend(
                codes
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
            );
        } else {
            blocker_codes.push("next_action_target_missing".to_string());
        }
    }
    if latest_run_graph_status
        .as_ref()
        .is_some_and(|status| status.status == "blocked")
    {
        blocker_codes.push("latest_run_graph_status_blocked".to_string());
    }
    let latest_run_graph_terminal_closure_has_truth = match latest_run_graph_status.as_ref() {
        Some(status)
            if crate::taskflow_run_graph_task_authority::run_graph_status_is_terminal_closure(
                status,
            ) =>
        {
            store
                .run_graph_terminal_closure_has_task_close_truth(status)
                .await
                .map_err(|error| {
                    format!("read latest run graph terminal closure evidence: {error}")
                })?
        }
        _ => false,
    };
    let latest_run_graph_terminal_closure_without_truth =
        latest_run_graph_status.as_ref().is_some_and(|status| {
            crate::taskflow_run_graph_task_authority::run_graph_status_is_terminal_closure(status)
                && !latest_run_graph_terminal_closure_has_truth
                && closed_task_ids.iter().any(|id| id == &status.task_id)
        });
    let latest_terminal_task_active_run_graph_task_stale =
        match latest_terminal_task_active_run_graph_status.as_ref() {
            Some(status)
                if crate::taskflow_run_graph_task_authority::terminal_task_active_status_matches_current_run(
                    latest_run_graph_status.as_ref(),
                    status,
                ) =>
            {
                let verdict =
                    crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(
                        &store, status,
                    )
                    .await
                    .map_err(|error| {
                        format!("read latest terminal-task authority verdict: {error}")
                    })?;
                verdict.task_missing() || verdict.task_closed_stale_run()
            }
            Some(_) => false,
            None => false,
        };
    let closed_task_active_run_projection_mismatch =
        post_commit_closed_task_active_run_projection_mismatch(
            latest_run_graph_status.as_ref(),
            latest_terminal_task_active_run_graph_task_stale,
            &closed_task_ids,
            latest_run_graph_terminal_closure_has_truth,
        ) || latest_run_graph_terminal_closure_without_truth;
    if closed_task_active_run_projection_mismatch {
        blocker_codes.push("closed_task_active_run_projection_mismatch".to_string());
    }
    blocker_codes.sort();
    blocker_codes.dedup();

    let status = status_from_blockers(&blocker_codes);
    let next_actions = if status == "pass" {
        Vec::<String>::new()
    } else if blocker_codes
        .iter()
        .any(|code| code == "closed_task_active_run_projection_mismatch")
    {
        vec![
            closed_task_active_run_projection_mismatch_next_action(),
            "If this is a VIDA runtime defect, search/comment/create only in the upstream VIDA stack issue tracker.".to_string(),
        ]
    } else {
        vec![
            "Inspect the blocked diagnostic sections before reporting closure.".to_string(),
            "If this is a VIDA runtime defect, search/comment/create only in the upstream VIDA stack issue tracker.".to_string(),
        ]
    };
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
        "next_actions": next_actions,
        "git_status": git_status,
        "taskflow_status": {
            "task_count": task_ids.len(),
            "latest_run_graph_status": latest_run_graph_status,
            "latest_terminal_task_active_run_graph_status": latest_terminal_task_active_run_graph_status,
            "latest_run_graph_recovery": latest_run_graph_recovery,
            "latest_dispatch_receipt": latest_dispatch_receipt,
            "latest_explicit_continuation_binding": latest_explicit_binding,
            "continuation_target_actionability": target_actionability,
            "closed_task_active_run_projection_mismatch": closed_task_active_run_projection_mismatch,
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
                    let blocker_names = blockers
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .collect::<Vec<_>>();
                    if !blocker_names.is_empty() {
                        println!("blockers: {}", blocker_names.join(", "));
                    }
                }
                if payload["status"].as_str() != Some("pass") {
                    if let Some(run_id) = payload["canonical_continuation_run_id"]
                        .as_str()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                    {
                        println!("run_id: {run_id}");
                    }
                    if let Some(command) = post_commit_default_clear_command(&payload) {
                        println!("clear_command: {command}");
                    }
                }
                if let Some(next_actions) = payload["next_actions"].as_array() {
                    let action_names = next_actions
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .take(2)
                        .collect::<Vec<_>>();
                    if !action_names.is_empty() {
                        println!("next:");
                        for action in action_names {
                            println!("  - {action}");
                        }
                    }
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
        DiagnosticsCommand::EvidenceCheck(args) => {
            run_diagnostics_gate(build_evidence_check_diagnostics(&args), args.json)
        }
        DiagnosticsCommand::RulesCheck(args) => {
            run_diagnostics_gate(build_rules_check_diagnostics(&args), args.json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_evidence_check_diagnostics, build_rules_check_diagnostics,
        closed_task_active_run_projection_mismatch_next_action,
        compact_host_dispatch_preflight_for_diagnostics, missing_task_actionability,
        post_commit_closed_task_active_run_projection_mismatch, post_commit_default_clear_command,
        run_post_commit, POST_COMMIT_DIAGNOSTICS_PROJECTION_NAME,
    };
    use crate::test_cli_support::guard_current_dir;
    use crate::{
        DiagnosticsEvidenceCheckArgs, DiagnosticsPostCommitArgs, DiagnosticsRulesCheckArgs,
    };
    use std::fs;
    use std::path::PathBuf;
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

    fn run_graph_status_for_diagnostics_test(
        task_id: &str,
        status: &str,
        lifecycle_stage: &str,
        resume_target: &str,
        next_node: Option<&str>,
    ) -> crate::state_store::RunGraphStatus {
        crate::state_store::RunGraphStatus {
            run_id: task_id.to_string(),
            task_id: task_id.to_string(),
            task_class: "runtime".to_string(),
            active_node: "closure".to_string(),
            next_node: next_node.map(str::to_string),
            status: status.to_string(),
            route_task_class: "runtime".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: task_id.to_string(),
            lifecycle_stage: lifecycle_stage.to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "ready".to_string(),
            checkpoint_kind: "closure".to_string(),
            resume_target: resume_target.to_string(),
            recovery_ready: false,
        }
    }

    #[test]
    fn diagnostics_post_commit_flags_closed_task_active_run_projection_mismatch() {
        let terminal_with_truth = run_graph_status_for_diagnostics_test(
            "closed-task",
            "completed",
            "closure_complete",
            "none",
            None,
        );
        let blocked_closed = run_graph_status_for_diagnostics_test(
            "closed-task",
            "blocked",
            "implementation_blocked",
            "implementer",
            Some("implementer"),
        );
        let closed_task_ids = vec!["closed-task".to_string(), "stale-closed-task".to_string()];

        assert!(!post_commit_closed_task_active_run_projection_mismatch(
            Some(&terminal_with_truth),
            false,
            &closed_task_ids,
            true,
        ));
        assert!(post_commit_closed_task_active_run_projection_mismatch(
            Some(&terminal_with_truth),
            true,
            &closed_task_ids,
            true,
        ));
        assert!(post_commit_closed_task_active_run_projection_mismatch(
            Some(&blocked_closed),
            false,
            &closed_task_ids,
            false,
        ));
        assert!(closed_task_active_run_projection_mismatch_next_action()
            .contains("vida task reconcile-closed-runs --limit 25"));
        assert!(!closed_task_active_run_projection_mismatch_next_action().contains("--json"));

        let payload = serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["latest_run_graph_status_blocked"],
            "canonical_continuation_run_id": "run-1"
        });
        assert_eq!(post_commit_default_clear_command(&payload), None);
        let payload = serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["closed_task_active_run_projection_mismatch"],
            "canonical_continuation_run_id": "run-1"
        });
        assert_eq!(
            post_commit_default_clear_command(&payload),
            Some("vida task reconcile-closed-runs --limit 25")
        );
        assert!(!post_commit_default_clear_command(&payload)
            .unwrap()
            .contains("--json"));
    }

    #[test]
    fn diagnostics_evidence_check_blocks_missing_evidence_refs() {
        let payload = build_evidence_check_diagnostics(&DiagnosticsEvidenceCheckArgs {
            task_id: Some("task-1".to_string()),
            ..Default::default()
        });

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["blocker_codes"][0], "missing_gate_evidence");
        assert_eq!(
            payload["vida_gate_result"]["status"],
            "insufficient_evidence"
        );
        assert_eq!(
            payload["vida_gate_result"]["issues"][0]["code"],
            "insufficient_evidence"
        );
    }

    #[test]
    fn diagnostics_evidence_check_passes_with_concrete_evidence_refs() {
        let payload = build_evidence_check_diagnostics(&DiagnosticsEvidenceCheckArgs {
            task_id: Some("task-1".to_string()),
            evidence_refs: vec![" cargo test -p vida diagnostics_surface ".to_string()],
            ..Default::default()
        });

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["blocker_codes"].as_array().unwrap().len(), 0);
        assert_eq!(payload["vida_gate_result"]["status"], "pass");
        assert_eq!(
            payload["vida_gate_result"]["evidence_refs"][0],
            "cargo test -p vida diagnostics_surface"
        );
    }

    #[test]
    fn diagnostics_rules_check_blocks_without_inputs() {
        let payload = build_rules_check_diagnostics(&DiagnosticsRulesCheckArgs::default());

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["blocker_codes"][0], "missing_gate_evidence");
        assert_eq!(
            payload["vida_gate_result"]["status"],
            "insufficient_evidence"
        );
    }

    #[test]
    fn diagnostics_rules_check_reports_missing_changed_path_as_gate_issue() {
        let payload = build_rules_check_diagnostics(&DiagnosticsRulesCheckArgs {
            changed_paths: vec![PathBuf::from("does/not/exist")],
            ..Default::default()
        });

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["blocker_codes"][0], "rules_check_path_missing");
        assert_eq!(
            payload["vida_gate_result"]["issues"][0]["code"],
            "rules_check_path_missing"
        );
    }

    #[test]
    fn diagnostics_rules_check_accepts_existing_protocol_id() {
        let payload = build_rules_check_diagnostics(&DiagnosticsRulesCheckArgs {
            protocol_ids: vec!["instruction-contracts/core.orchestration-protocol".to_string()],
            ..Default::default()
        });

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["vida_gate_result"]["status"], "pass");
        assert_eq!(
            payload["vida_gate_result"]["evidence_refs"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
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
            missing_task_actionability(Some(&recovery), None, &["other-task".to_string()], &[]);
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["blocker_codes"][0], "next_action_target_missing");
        assert!(payload["next_actions"][0].as_str().is_some_and(|action| {
            action.contains("missing-task")
                && action.contains("vida taskflow recovery status run-1")
                && !action.contains("vida taskflow recovery status run-1 --json")
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
            &["closed-run".to_string()],
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

        let payload = missing_task_actionability(Some(&recovery), None, &[], &[]);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["checked_task_id"], "runtime-vida-taskflow-codex");
        assert_eq!(payload["checked_source"], "run_graph_recovery");
        assert_eq!(payload["terminal_runtime_run_without_task"], true);
    }

    #[test]
    fn diagnostics_allows_explicit_binding_to_closed_task_when_recovery_is_terminal_closure() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "closed-run".to_string(),
            task_id: "closed-task".to_string(),
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
        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "closed-run".to_string(),
            task_id: "closed-task".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "task_graph_task",
                "run_id": "closed-run",
                "task_id": "closed-task",
                "task_status": "closed"
            }),
            binding_source: "task_close_reconcile".to_string(),
            why_this_unit: "test terminal closure binding".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: None,
            recorded_at: "2026-06-22T16:00:00Z".to_string(),
        };

        let payload = missing_task_actionability(
            Some(&recovery),
            Some(&binding),
            &["closed-task".to_string()],
            &["closed-task".to_string()],
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["checked_task_id"], "closed-task");
        assert_eq!(payload["checked_source"], "explicit_continuation_binding");
        assert_eq!(payload["terminal_closure_recovery"], true);
    }

    #[test]
    fn diagnostics_blocks_closed_binding_when_recovery_belongs_to_other_run() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "other-run".to_string(),
            task_id: "closed-task".to_string(),
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
        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "closed-run".to_string(),
            task_id: "closed-task".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "task_graph_task",
                "run_id": "closed-run",
                "task_id": "closed-task",
                "task_status": "closed"
            }),
            binding_source: "task_close_reconcile".to_string(),
            why_this_unit: "test mismatched recovery".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: None,
            recorded_at: "2026-06-22T16:40:00Z".to_string(),
        };

        let payload = missing_task_actionability(
            Some(&recovery),
            Some(&binding),
            &["closed-task".to_string()],
            &["closed-task".to_string()],
        );

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"][0],
            "closed_task_active_run_projection_mismatch"
        );
        assert!(payload.get("terminal_closure_recovery").is_none());
    }

    #[test]
    fn diagnostics_blocks_continuation_bind_to_closed_task() {
        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "closed-run".to_string(),
            task_id: "closed-task".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "task_graph_task",
                "run_id": "closed-run",
                "task_id": "closed-task",
                "task_status": "closed"
            }),
            binding_source: "explicit_continuation_bind_task".to_string(),
            why_this_unit: "test closed binding".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
            request_text: Some("Continue closed task".to_string()),
            recorded_at: "2026-06-13T00:00:00Z".to_string(),
        };

        let payload = missing_task_actionability(
            None,
            Some(&binding),
            &["closed-task".to_string()],
            &["closed-task".to_string()],
        );

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"][0],
            "closed_task_active_run_projection_mismatch"
        );
        assert_eq!(payload["checked_task_id"], "closed-task");
        assert_eq!(payload["task_status"], "closed");
        assert!(payload["next_actions"][0]
            .as_str()
            .is_some_and(|action| action.contains("vida task reconcile-closed-runs --limit 25")));
    }
}
