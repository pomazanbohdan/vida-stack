use std::process::ExitCode;

use serde_json::json;
use vida_contracts::{
    operations, VidaClaimKind, VidaClientKind, VidaCommandEnvelope, VidaCommandResponse,
    VidaIdempotencyKey, VidaOperation, VidaProjectId, VidaProjectRef, VidaRequestId, VidaSessionId,
    VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION,
};

use crate::vida_client::VidaClient;
use crate::vida_client_inprocess::InProcessVidaClient;
use crate::ProxyArgs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ServiceCliFamily {
    Service,
    Project,
    Wizard,
    Job,
    Receipt,
}

#[derive(Debug, Clone)]
pub(crate) struct ServiceCliRequest {
    pub(crate) family: ServiceCliFamily,
    pub(crate) command: String,
    pub(crate) operation: &'static str,
    pub(crate) project_ref: Option<VidaProjectRef>,
    pub(crate) payload: serde_json::Value,
    pub(crate) as_json: bool,
}

pub(crate) fn run_service(args: ProxyArgs) -> ExitCode {
    run_with_client(
        ServiceCliFamily::Service,
        args,
        &InProcessVidaClient::new_ready(),
    )
}

pub(crate) fn run_project(args: ProxyArgs) -> ExitCode {
    run_with_client(
        ServiceCliFamily::Project,
        args,
        &InProcessVidaClient::new_ready(),
    )
}

pub(crate) fn run_wizard(args: ProxyArgs) -> ExitCode {
    run_with_client(
        ServiceCliFamily::Wizard,
        args,
        &InProcessVidaClient::new_ready(),
    )
}

pub(crate) fn run_job(args: ProxyArgs) -> ExitCode {
    run_with_client(
        ServiceCliFamily::Job,
        args,
        &InProcessVidaClient::new_ready(),
    )
}

pub(crate) fn run_receipt(args: ProxyArgs) -> ExitCode {
    run_with_client(
        ServiceCliFamily::Receipt,
        args,
        &InProcessVidaClient::new_ready(),
    )
}

pub(crate) fn run_with_client<C: VidaClient>(
    family: ServiceCliFamily,
    args: ProxyArgs,
    client: &C,
) -> ExitCode {
    let request = match service_cli_request(family, &args.args) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let response = execute_service_cli_request(client, &request);
    emit_service_client_response(&request, &response)
}

pub(crate) fn execute_service_cli_request<C: VidaClient>(
    client: &C,
    request: &ServiceCliRequest,
) -> VidaCommandResponse {
    client.execute(envelope_for_request(request))
}

pub(crate) fn service_cli_request(
    family: ServiceCliFamily,
    args: &[String],
) -> Result<ServiceCliRequest, String> {
    let command = args
        .iter()
        .find(|arg| !arg.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("status");
    let as_json = args.iter().any(|arg| arg == "--json");
    let project_ref = project_ref_from_args(args);
    let operation = match (&family, command) {
        (ServiceCliFamily::Service, "hello") => operations::SERVICE_HELLO,
        (ServiceCliFamily::Service, "status") => operations::SERVICE_STATUS,
        (ServiceCliFamily::Service, "capabilities") => operations::SERVICE_CAPABILITIES,
        (ServiceCliFamily::Service, "endpoints")
        | (ServiceCliFamily::Service, "endpoint-status") => operations::SERVICE_ENDPOINT_STATUS,
        (ServiceCliFamily::Service, "events") => operations::EVENTS_SINCE,
        (ServiceCliFamily::Project, "list") => operations::PROJECT_REGISTRY_LIST,
        (ServiceCliFamily::Project, "resolve") => operations::PROJECT_RESOLVE,
        (ServiceCliFamily::Project, "status") => operations::PROJECT_STATUS,
        (ServiceCliFamily::Wizard, "inspect") => operations::WIZARD_SCHEMA_GET,
        (ServiceCliFamily::Wizard, "draft") => operations::WIZARD_SESSION_START,
        (ServiceCliFamily::Wizard, "validate") => operations::WIZARD_SESSION_VALIDATE,
        (ServiceCliFamily::Wizard, "diff") => operations::WIZARD_SESSION_DIFF,
        (ServiceCliFamily::Job, "status") => operations::JOBS_GET,
        (ServiceCliFamily::Receipt, "get") => operations::RECEIPTS_GET,
        _ => {
            return Err(format!(
                "Unsupported service-client command `{command}` for family `{}`.",
                family_name(&family)
            ));
        }
    };
    Ok(ServiceCliRequest {
        family,
        command: command.to_string(),
        operation,
        project_ref,
        payload: payload_for(operation, args),
        as_json,
    })
}

fn envelope_for_request(request: &ServiceCliRequest) -> VidaCommandEnvelope {
    VidaCommandEnvelope {
        schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
        protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
        operation: VidaOperation(request.operation.to_string()),
        session_id: VidaSessionId("cli-service-session".to_string()),
        request_id: VidaRequestId(format!(
            "cli-{}-{}",
            family_name(&request.family),
            request.command
        )),
        client_kind: VidaClientKind::Cli,
        project_ref: request.project_ref.clone(),
        claim_kind: Some(VidaClaimKind::SharedRead),
        payload: request.payload.clone(),
        correlation: Some(json!({
            "source": "vida_cli_service_client",
            "family": family_name(&request.family),
            "command": request.command
        })),
        idempotency_key: Some(VidaIdempotencyKey(format!(
            "cli-{}-{}",
            family_name(&request.family),
            request.command
        ))),
        apply_token: None,
    }
}

fn payload_for(operation: &str, args: &[String]) -> serde_json::Value {
    match operation {
        operations::EVENTS_SINCE => {
            json!({ "cursor": value_after(args, "--cursor").unwrap_or("latest") })
        }
        operations::PROJECT_REGISTRY_GET
        | operations::PROJECT_RESOLVE
        | operations::PROJECT_STATUS => {
            json!({ "project": value_after(args, "--project").unwrap_or("vida-stack") })
        }
        operations::WIZARD_SCHEMA_GET => {
            json!({ "wizard_kind": value_after(args, "--kind").unwrap_or("project_init") })
        }
        operations::WIZARD_SESSION_START
        | operations::WIZARD_SESSION_VALIDATE
        | operations::WIZARD_SESSION_DIFF => json!({
            "wizard_kind": value_after(args, "--kind").unwrap_or("project_init"),
            "dry_run": true
        }),
        operations::JOBS_GET => json!({ "job_id": value_after(args, "--job").unwrap_or("latest") }),
        operations::RECEIPTS_GET => {
            json!({ "receipt_id": value_after(args, "--receipt").unwrap_or("latest") })
        }
        _ => json!({}),
    }
}

fn project_ref_from_args(args: &[String]) -> Option<VidaProjectRef> {
    value_after(args, "--project").map(|project_id| VidaProjectRef::ProjectId {
        project_id: VidaProjectId(project_id.to_string()),
    })
}

fn value_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].as_str())
}

fn emit_service_client_response(
    request: &ServiceCliRequest,
    response: &VidaCommandResponse,
) -> ExitCode {
    if request.as_json {
        crate::print_json_pretty(&json!({
            "status": response.status,
            "family": family_name(&request.family),
            "command": request.command,
            "operation": request.operation,
            "response": response
        }));
    } else {
        println!(
            "VIDA {} {} -> {:?}",
            family_name(&request.family),
            request.command,
            response.status
        );
    }
    match response.error {
        Some(_) => ExitCode::from(1),
        None => ExitCode::SUCCESS,
    }
}

fn family_name(family: &ServiceCliFamily) -> &'static str {
    match family {
        ServiceCliFamily::Service => "service",
        ServiceCliFamily::Project => "project",
        ServiceCliFamily::Wizard => "wizard",
        ServiceCliFamily::Job => "job",
        ServiceCliFamily::Receipt => "receipt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vida_client::pass_response;
    use std::cell::RefCell;

    struct RecordingClient {
        operations: RefCell<Vec<String>>,
    }

    impl RecordingClient {
        fn new() -> Self {
            Self {
                operations: RefCell::new(Vec::new()),
            }
        }
    }

    impl VidaClient for RecordingClient {
        fn execute(&self, envelope: VidaCommandEnvelope) -> VidaCommandResponse {
            self.operations
                .borrow_mut()
                .push(envelope.operation.0.clone());
            pass_response(&envelope, json!({ "recorded": envelope.operation.0 }))
        }
    }

    #[test]
    fn cli_service_client_routes_all_service_first_families_through_vida_client() {
        let cases = [
            (
                ServiceCliFamily::Service,
                vec!["status"],
                operations::SERVICE_STATUS,
            ),
            (
                ServiceCliFamily::Service,
                vec!["endpoints"],
                operations::SERVICE_ENDPOINT_STATUS,
            ),
            (
                ServiceCliFamily::Service,
                vec!["capabilities"],
                operations::SERVICE_CAPABILITIES,
            ),
            (
                ServiceCliFamily::Service,
                vec!["events"],
                operations::EVENTS_SINCE,
            ),
            (
                ServiceCliFamily::Project,
                vec!["list"],
                operations::PROJECT_REGISTRY_LIST,
            ),
            (
                ServiceCliFamily::Project,
                vec!["resolve", "--project", "fixture-project"],
                operations::PROJECT_RESOLVE,
            ),
            (
                ServiceCliFamily::Project,
                vec!["status", "--project", "fixture-project"],
                operations::PROJECT_STATUS,
            ),
            (
                ServiceCliFamily::Wizard,
                vec!["inspect"],
                operations::WIZARD_SCHEMA_GET,
            ),
            (
                ServiceCliFamily::Wizard,
                vec!["draft"],
                operations::WIZARD_SESSION_START,
            ),
            (
                ServiceCliFamily::Wizard,
                vec!["validate"],
                operations::WIZARD_SESSION_VALIDATE,
            ),
            (
                ServiceCliFamily::Wizard,
                vec!["diff"],
                operations::WIZARD_SESSION_DIFF,
            ),
            (ServiceCliFamily::Job, vec!["status"], operations::JOBS_GET),
            (
                ServiceCliFamily::Receipt,
                vec!["get"],
                operations::RECEIPTS_GET,
            ),
        ];
        let client = RecordingClient::new();

        for (family, args, expected_operation) in cases {
            let args = args.into_iter().map(String::from).collect::<Vec<_>>();
            let request = service_cli_request(family, &args).expect("request should map");
            assert_eq!(request.operation, expected_operation);
            let response = execute_service_cli_request(&client, &request);
            assert!(response.error.is_none());
        }

        let operations = client.operations.borrow().clone();
        assert_eq!(operations.len(), 13);
        assert!(operations.contains(&operations::SERVICE_STATUS.to_string()));
        assert!(operations.contains(&operations::PROJECT_STATUS.to_string()));
        assert!(operations.contains(&operations::WIZARD_SESSION_DIFF.to_string()));
        assert!(operations.contains(&operations::JOBS_GET.to_string()));
        assert!(operations.contains(&operations::RECEIPTS_GET.to_string()));
    }

    #[test]
    fn cli_service_client_rejects_unknown_family_command_before_client_execution() {
        let err = service_cli_request(ServiceCliFamily::Wizard, &[String::from("apply")])
            .expect_err("unsupported command should fail closed");
        assert!(err.contains("Unsupported service-client command"));
    }
}
