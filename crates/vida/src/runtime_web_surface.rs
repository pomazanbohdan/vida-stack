use std::process::ExitCode;

use serde_json::Value;

use crate::operator_contracts::{
    finalize_release1_operator_truth, shared_operator_output_contract_parity_error,
};
use crate::release1_contracts::{blocker_code_str, BlockerCode};
use crate::surface_render::print_surface_json;
use crate::{
    print_surface_header, print_surface_line, RuntimeArgs, RuntimeCommand, RuntimeWebCommand,
    RuntimeWebRestartArgs,
};

const RUNTIME_WEB_RESTART_SURFACE: &str = "vida runtime web restart";
const RESTART_EXECUTOR_BLOCKER: BlockerCode = BlockerCode::ToolContractMissing;
const RESTART_EXECUTOR_NEXT_ACTION: &str = "Add ownership-safe process discovery and restart executor support, then rerun `vida runtime web restart --scope current-repo --include-edge-proxy --json`.";

pub(crate) async fn run_runtime(args: RuntimeArgs) -> ExitCode {
    match args.command {
        RuntimeCommand::Web(args) => match args.command {
            RuntimeWebCommand::Restart(args) => run_runtime_web_restart(args),
        },
    }
}

fn run_runtime_web_restart(args: RuntimeWebRestartArgs) -> ExitCode {
    let payload = build_runtime_web_restart_payload(&args);
    if print_surface_json(
        &payload,
        args.json,
        "runtime web restart payload should render as json",
    ) {
        return ExitCode::from(1);
    }

    if !args.json {
        print_runtime_web_restart_plain(&payload);
    }

    if payload["status"].as_str() == Some("pass") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn build_runtime_web_restart_payload(args: &RuntimeWebRestartArgs) -> Value {
    let project_root = crate::resolve_runtime_project_root()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|error| format!("unresolved: {error}"));
    let dry_run = args.dry_run;
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    if !dry_run {
        blocker_codes.push(blocker_code_str(RESTART_EXECUTOR_BLOCKER).to_string());
        next_actions.push(RESTART_EXECUTOR_NEXT_ACTION.to_string());
    }

    let actions = runtime_web_restart_actions(args.include_edge_proxy, dry_run);
    let blocked_components =
        runtime_web_restart_blocked_components(args.include_edge_proxy, dry_run);
    let components = runtime_web_restart_components(args.include_edge_proxy, dry_run);
    let artifact_refs = serde_json::json!({
        "surface": RUNTIME_WEB_RESTART_SURFACE,
        "scope": args.scope,
        "include_edge_proxy": args.include_edge_proxy,
        "dry_run": dry_run,
        "project_root": project_root,
    });
    let finalized = finalize_release1_operator_truth(blocker_codes, next_actions, artifact_refs)
        .expect("runtime web restart operator truth should finalize");
    let mut payload = serde_json::json!({
        "surface": RUNTIME_WEB_RESTART_SURFACE,
        "status": finalized.status,
        "trace_id": finalized.operator_contracts["trace_id"].clone(),
        "workflow_class": finalized.operator_contracts["workflow_class"].clone(),
        "risk_tier": finalized.operator_contracts["risk_tier"].clone(),
        "blocker_codes": finalized.blocker_codes,
        "next_actions": finalized.next_actions,
        "artifact_refs": finalized.artifact_refs,
        "shared_fields": finalized.shared_fields,
        "operator_contracts": finalized.operator_contracts,
        "restart": {
            "scope": args.scope,
            "include_edge_proxy": args.include_edge_proxy,
            "dry_run": dry_run,
            "project_root": project_root,
            "mode": if dry_run { "plan_only" } else { "blocked_until_executor_available" },
            "actions": actions,
            "blocked_components": blocked_components,
            "components": components,
        },
        "actions": actions,
        "blocked_components": blocked_components,
    });
    for key in ["trace_id", "workflow_class", "risk_tier"] {
        payload["shared_fields"][key] = payload["operator_contracts"][key].clone();
    }
    assert_eq!(
        shared_operator_output_contract_parity_error(&payload),
        None,
        "runtime web restart payload should keep release-1 parity"
    );
    payload
}

fn runtime_web_restart_actions(include_edge_proxy: bool, dry_run: bool) -> Value {
    let restart_action = if dry_run { "planned" } else { "blocked" };
    let edge_action = if include_edge_proxy {
        restart_action
    } else {
        "skipped"
    };
    serde_json::json!([
        { "component_id": "local_web_upstream", "action": restart_action },
        { "component_id": "local_proxy", "action": restart_action },
        { "component_id": "edge_proxy", "action": edge_action },
    ])
}

fn runtime_web_restart_blocked_components(include_edge_proxy: bool, dry_run: bool) -> Value {
    if dry_run {
        return serde_json::json!([]);
    }
    let mut blocked = vec!["local_web_upstream", "local_proxy"];
    if include_edge_proxy {
        blocked.push("edge_proxy");
    }
    serde_json::json!(blocked)
}

fn runtime_web_restart_components(include_edge_proxy: bool, dry_run: bool) -> Value {
    let restart_action = if dry_run { "planned" } else { "blocked" };
    let edge_action = if include_edge_proxy {
        restart_action
    } else {
        "skipped"
    };
    serde_json::json!([
        {
            "id": "local_web_upstream",
            "kind": "web_upstream",
            "action": restart_action,
            "ownership": "current_repo_required",
            "ports": [],
            "blocker_code": if dry_run { Value::Null } else { Value::String(blocker_code_str(RESTART_EXECUTOR_BLOCKER).to_string()) },
        },
        {
            "id": "local_proxy",
            "kind": "proxy",
            "action": restart_action,
            "ownership": "current_repo_required",
            "ports": [],
            "blocker_code": if dry_run { Value::Null } else { Value::String(blocker_code_str(RESTART_EXECUTOR_BLOCKER).to_string()) },
        },
        {
            "id": "edge_proxy",
            "kind": "edge_proxy",
            "action": edge_action,
            "ownership": "explicit_include_required",
            "ports": [],
            "blocker_code": if !include_edge_proxy || dry_run { Value::Null } else { Value::String(blocker_code_str(RESTART_EXECUTOR_BLOCKER).to_string()) },
        }
    ])
}

fn print_runtime_web_restart_plain(payload: &Value) {
    print_surface_header(crate::RenderMode::Plain, "Runtime web restart");
    print_surface_line(
        crate::RenderMode::Plain,
        "status",
        payload["status"].as_str().unwrap_or("blocked"),
    );
    print_surface_line(
        crate::RenderMode::Plain,
        "scope",
        payload["restart"]["scope"].as_str().unwrap_or(""),
    );
    print_surface_line(
        crate::RenderMode::Plain,
        "mode",
        payload["restart"]["mode"].as_str().unwrap_or("unknown"),
    );
    if let Some(next_action) = payload["next_actions"]
        .as_array()
        .and_then(|values| values.first())
        .and_then(|value| value.as_str())
    {
        print_surface_line(crate::RenderMode::Plain, "next action", next_action);
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;
    use crate::{Cli, Command};

    #[test]
    fn runtime_web_restart_cli_accepts_current_repo_edge_proxy_dry_run_json() {
        let cli = Cli::try_parse_from([
            "vida",
            "runtime",
            "web",
            "restart",
            "--scope",
            "current-repo",
            "--include-edge-proxy",
            "--dry-run",
            "--json",
        ])
        .expect("runtime web restart should parse");

        let Some(Command::Runtime(args)) = cli.command else {
            panic!("runtime command should parse as root runtime command");
        };
        let RuntimeCommand::Web(web) = args.command;
        let RuntimeWebCommand::Restart(restart) = web.command;
        assert_eq!(restart.scope, "current-repo");
        assert!(restart.include_edge_proxy);
        assert!(restart.dry_run);
        assert!(restart.json);
    }

    #[test]
    fn runtime_web_restart_help_documents_options() {
        let help = Cli::try_parse_from(["vida", "runtime", "web", "restart", "--help"])
            .expect_err("help should render clap display error")
            .to_string();

        for expected in [
            "restart current-repo web proof listeners with fail-closed ownership checks",
            "--scope <SCOPE>",
            "current-repo",
            "--include-edge-proxy",
            "--dry-run",
            "--json",
        ] {
            assert!(
                help.contains(expected),
                "runtime web restart help should document `{expected}`:\n{help}"
            );
        }
    }

    #[test]
    fn runtime_web_restart_dry_run_payload_is_standardized_pass_plan() {
        let payload = build_runtime_web_restart_payload(&RuntimeWebRestartArgs {
            scope: "current-repo".to_string(),
            include_edge_proxy: true,
            dry_run: true,
            json: true,
        });

        assert_eq!(payload["surface"], RUNTIME_WEB_RESTART_SURFACE);
        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["restart"]["mode"], "plan_only");
        assert_eq!(
            payload["restart"]["blocked_components"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert_eq!(payload["actions"][0]["action"], "planned");
        assert_eq!(payload["restart"]["components"][2]["action"], "planned");
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn runtime_web_restart_without_executor_fails_closed_for_mutation() {
        let payload = build_runtime_web_restart_payload(&RuntimeWebRestartArgs {
            scope: "current-repo".to_string(),
            include_edge_proxy: false,
            dry_run: false,
            json: true,
        });

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"][0],
            blocker_code_str(RESTART_EXECUTOR_BLOCKER)
        );
        assert_eq!(payload["blocked_components"][0], "local_web_upstream");
        assert_eq!(payload["restart"]["components"][0]["action"], "blocked");
        assert_eq!(payload["restart"]["components"][2]["action"], "skipped");
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }
}
