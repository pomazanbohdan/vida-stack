use std::process::ExitCode;

use serde_json::Value;

use crate::operator_contracts::{
    finalize_release1_operator_truth, shared_operator_output_contract_parity_error,
};
use crate::release1_contracts::{blocker_code_str, BlockerCode};
use crate::surface_render::print_surface_json;
use crate::{print_surface_header, print_surface_line, ProofArgs, ProofCommand};

const BROWSER_PROOF_SURFACE: &str = "vida proof browser";
const BROWSER_AUTOMATION_BLOCKER: BlockerCode = BlockerCode::ToolContractMissing;
const BROWSER_AUTOMATION_NEXT_ACTION: &str = "Configure a browser proof adapter or use a host browser proof collection surface that can produce screenshot, DOM, console, route, and proxy-health artifacts, then rerun `vida proof browser --route <route> --expect <text> --json`.";

pub(crate) async fn run_proof(args: ProofArgs) -> ExitCode {
    match args.command {
        ProofCommand::Browser(args) => run_browser_proof(args),
    }
}

fn run_browser_proof(args: crate::ProofBrowserArgs) -> ExitCode {
    let payload = build_browser_proof_payload(&args.route, args.expect.as_deref());
    if print_surface_json(
        &payload,
        args.json,
        "browser proof payload should render as json",
    ) {
        return ExitCode::from(1);
    }

    print_browser_proof_plain(&payload);
    ExitCode::from(1)
}

fn build_browser_proof_payload(route: &str, expected_text: Option<&str>) -> Value {
    let blocker_codes = vec![blocker_code_str(BROWSER_AUTOMATION_BLOCKER).to_string()];
    let next_actions = vec![BROWSER_AUTOMATION_NEXT_ACTION.to_string()];
    let artifact_refs = serde_json::json!({
        "surface": BROWSER_PROOF_SURFACE,
        "route": route,
        "expected_text": expected_text,
        "screenshot": null,
        "dom_snapshot": null,
        "console_log": null,
        "app_route": null,
        "proxy_health": null,
    });
    let finalized = finalize_release1_operator_truth(blocker_codes, next_actions, artifact_refs)
        .expect("browser proof operator truth should finalize");
    let mut payload = serde_json::json!({
        "surface": BROWSER_PROOF_SURFACE,
        "status": finalized.status,
        "trace_id": finalized.operator_contracts["trace_id"].clone(),
        "workflow_class": finalized.operator_contracts["workflow_class"].clone(),
        "risk_tier": finalized.operator_contracts["risk_tier"].clone(),
        "blocker_codes": finalized.blocker_codes,
        "next_actions": finalized.next_actions,
        "artifact_refs": finalized.artifact_refs,
        "shared_fields": finalized.shared_fields,
        "operator_contracts": finalized.operator_contracts,
        "proof": {
            "kind": "browser",
            "route": route,
            "expected_text": expected_text,
            "result": "blocked",
            "collection_state": "browser_automation_unavailable",
            "artifacts": {
                "screenshot": null,
                "dom_snapshot": null,
                "console_log": null,
                "app_route": null,
                "proxy_health": null,
            }
        }
    });
    for key in ["trace_id", "workflow_class", "risk_tier"] {
        payload["shared_fields"][key] = payload["operator_contracts"][key].clone();
    }
    assert_eq!(
        shared_operator_output_contract_parity_error(&payload),
        None,
        "browser proof payload should keep release-1 parity"
    );
    payload
}

fn print_browser_proof_plain(payload: &Value) {
    print_surface_header(crate::RenderMode::Plain, "Browser proof");
    print_surface_line(
        crate::RenderMode::Plain,
        "status",
        payload["status"].as_str().unwrap_or("blocked"),
    );
    print_surface_line(
        crate::RenderMode::Plain,
        "route",
        payload["proof"]["route"].as_str().unwrap_or(""),
    );
    if let Some(expected_text) = payload["proof"]["expected_text"].as_str() {
        print_surface_line(crate::RenderMode::Plain, "expect", expected_text);
    }
    print_surface_line(
        crate::RenderMode::Plain,
        "collection state",
        payload["proof"]["collection_state"]
            .as_str()
            .unwrap_or("unknown"),
    );
    let blocker_codes = payload["blocker_codes"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    print_surface_line(crate::RenderMode::Plain, "blocker codes", &blocker_codes);
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
    use crate::{Cli, Command, ProofCommand};

    #[test]
    fn proof_browser_cli_accepts_route_expect_and_json() {
        let cli = Cli::try_parse_from([
            "vida",
            "proof",
            "browser",
            "--route",
            "http://127.0.0.1:51235/#/module/project",
            "--expect",
            "My Tasks",
            "--json",
        ])
        .expect("proof browser command should parse");

        let Some(Command::Proof(args)) = cli.command else {
            panic!("proof command should parse as root proof command");
        };
        let ProofCommand::Browser(browser) = args.command;
        assert_eq!(browser.route, "http://127.0.0.1:51235/#/module/project");
        assert_eq!(browser.expect.as_deref(), Some("My Tasks"));
        assert!(browser.json);
    }

    #[test]
    fn proof_browser_payload_is_standardized_blocked_operator_output() {
        let payload = build_browser_proof_payload(
            "http://127.0.0.1:51235/#/module/project",
            Some("My Tasks"),
        );

        assert_eq!(payload["surface"], BROWSER_PROOF_SURFACE);
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["proof"]["result"], "blocked");
        assert_eq!(
            payload["proof"]["collection_state"],
            "browser_automation_unavailable"
        );
        assert_eq!(
            payload["blocker_codes"][0],
            blocker_code_str(BROWSER_AUTOMATION_BLOCKER)
        );
        assert!(payload["operator_contracts"].is_object());
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }
}
