use std::process::ExitCode;

use serde_json::json;
use vida_contracts::{
    operation_input_schema, operation_spec, operations, VidaClientKind, VidaCommandEnvelope,
    VidaCommandResponse, VidaIdempotencyKey, VidaOperation, VidaOperationInputField,
    VidaOperationInputValueKind, VidaProjectId, VidaProjectRef, VidaRequestId, VidaSessionId,
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
    if service_cli_help_requested(&args.args) {
        let args_without_help = args
            .args
            .iter()
            .filter(|arg| !matches!(arg.as_str(), "--help" | "-h"))
            .cloned()
            .collect::<Vec<_>>();
        let request = match service_cli_request(family, &args_without_help) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(2);
            }
        };
        print_service_operation_help(&request);
        return ExitCode::SUCCESS;
    }

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

fn service_cli_help_requested(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h"))
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
    let operation = match (&family, command) {
        (ServiceCliFamily::Service, "hello") => operations::SERVICE_HELLO,
        (ServiceCliFamily::Service, "status") => operations::SERVICE_STATUS,
        (ServiceCliFamily::Service, "capabilities") => operations::SERVICE_CAPABILITIES,
        (ServiceCliFamily::Service, "endpoints")
        | (ServiceCliFamily::Service, "endpoint-status") => operations::SERVICE_ENDPOINT_STATUS,
        (ServiceCliFamily::Service, "lifecycle-plan") => operations::SERVICE_LIFECYCLE_PLAN,
        (ServiceCliFamily::Service, "lifecycle-status") => operations::SERVICE_LIFECYCLE_STATUS,
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
    let project_ref = project_ref_from_operation(operation, args);
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
        command_id: None,
        causation_id: None,
        expected_stream_version: None,
        consistency: None,
        deadline: None,
        client_kind: VidaClientKind::Cli,
        project_ref: request.project_ref.clone(),
        claim_kind: operation_spec(request.operation).map(|spec| spec.required_claim),
        trusted_owned_path: None,
        trusted_owned_write_scopes: Vec::new(),
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
    let Some(schema) = operation_input_schema(operation) else {
        return json!({});
    };
    let mut payload = serde_json::Map::new();
    for field in schema.fields {
        if let Some(value) = payload_value_for_field(&field, args) {
            payload.insert(field.payload_key, value);
        }
    }
    serde_json::Value::Object(payload)
}

fn project_ref_from_operation(operation: &str, args: &[String]) -> Option<VidaProjectRef> {
    let schema = operation_input_schema(operation)?;
    let project_field = schema.field("project")?;
    let project = project_field
        .cli_flag
        .as_deref()
        .and_then(|flag| value_after(args, flag))
        .map(str::to_string)
        .or_else(|| project_field.default_value.clone());
    project.map(|project_id| VidaProjectRef::ProjectId {
        project_id: VidaProjectId(project_id.to_string()),
    })
}

fn payload_value_for_field(
    field: &VidaOperationInputField,
    args: &[String],
) -> Option<serde_json::Value> {
    let raw = field
        .cli_flag
        .as_deref()
        .and_then(|flag| value_after(args, flag))
        .map(str::to_string)
        .or_else(|| {
            if matches!(field.value_kind, VidaOperationInputValueKind::Boolean)
                && field
                    .cli_flag
                    .as_deref()
                    .is_some_and(|flag| flag_present(args, flag))
            {
                Some("true".to_string())
            } else {
                field.default_value.clone()
            }
        })?;
    Some(parse_schema_field_value(field.value_kind, &raw))
}

fn parse_schema_field_value(kind: VidaOperationInputValueKind, raw: &str) -> serde_json::Value {
    match kind {
        VidaOperationInputValueKind::Boolean => json!(matches!(
            raw.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )),
        VidaOperationInputValueKind::JsonObject => {
            serde_json::from_str(raw).unwrap_or_else(|_| json!({}))
        }
        VidaOperationInputValueKind::String
        | VidaOperationInputValueKind::Path
        | VidaOperationInputValueKind::EnumOne => json!(raw),
    }
}

fn value_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].as_str())
}

fn flag_present(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
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
        emit_default_service_client_response(request, response);
    }
    match response.error {
        Some(_) => ExitCode::from(1),
        None => ExitCode::SUCCESS,
    }
}

fn emit_default_service_client_response(
    request: &ServiceCliRequest,
    response: &VidaCommandResponse,
) {
    let status = format!("{:?}", response.status).to_ascii_lowercase();
    if request.family == ServiceCliFamily::Job {
        println!("vida job {}", request.command);
        println!("  status: {status}");
        if let Some(result) = &response.result {
            print_result_field(result, "job_id");
            print_result_field_as(result, "status", "job_status");
            print_result_field(result, "authority");
            print_result_field(result, "runner");
            if let Some(next_action) = result
                .get("job")
                .and_then(|job| job.get("next_action"))
                .and_then(serde_json::Value::as_str)
            {
                println!("  next_action: {}", terminal_safe_text(next_action));
            }
            if let Some(blocker) = result.get("job").and_then(|job| job.get("blocker")) {
                print_result_field(blocker, "code");
                print_result_field(blocker, "repair_action");
            }
        }
        return;
    }

    if request.family == ServiceCliFamily::Service && request.command == "capabilities" {
        println!("vida service capabilities");
        println!("  status: {status}");
        if let Some(result) = &response.result {
            if let Some(engine) = result.get("engine_capabilities") {
                print_result_field(engine, "engine_id");
                print_result_field(engine, "engine_kind");
                print_result_field_as(engine, "contract_version", "engine_contract");
                if let Some(capabilities) = engine
                    .get("capabilities")
                    .and_then(serde_json::Value::as_array)
                {
                    println!(
                        "  capabilities[{}]{{capability,supported,mode,blocker_code}}:",
                        capabilities.len()
                    );
                    for capability in capabilities {
                        println!(
                            "    {},{},{},{}",
                            json_field_as_str(capability, "capability"),
                            json_field_as_bool_string(capability, "supported"),
                            json_field_as_str(capability, "mode"),
                            json_field_as_str(capability, "blocker_code")
                        );
                    }
                }
            }
        }
        return;
    }

    println!(
        "VIDA {} {} -> {:?}",
        family_name(&request.family),
        request.command,
        response.status
    );
}

fn print_service_operation_help(request: &ServiceCliRequest) {
    println!("vida {} {}", family_name(&request.family), request.command);
    println!("  operation: {}", request.operation);
    let schema = operation_input_schema(request.operation);
    let fields = schema
        .as_ref()
        .map(|schema| schema.fields.as_slice())
        .unwrap_or(&[]);
    println!(
        "  inputs[{}]{{field_id,label,required,default,cli,control}}:",
        fields.len()
    );
    for field in fields {
        print_operation_field_help(field);
    }
    println!("  --json    Emit machine-readable JSON output");
}

fn print_operation_field_help(field: &VidaOperationInputField) {
    let cli = field.cli_flag.as_deref().unwrap_or("");
    let control =
        serde_json::to_value(field.tui_control).expect("TUI control should serialize to JSON");
    let control = control.as_str().unwrap_or("");
    println!(
        "    {},{},{},{},{},{}",
        terminal_safe_text(&field.field_id),
        terminal_safe_text(&field.label),
        field.required,
        terminal_safe_text(field.default_value.as_deref().unwrap_or("")),
        terminal_safe_text(&cli),
        control
    );
}

fn print_result_field(result: &serde_json::Value, field: &str) {
    print_result_field_as(result, field, field);
}

fn print_result_field_as(result: &serde_json::Value, field: &str, label: &str) {
    if let Some(value) = result.get(field).and_then(serde_json::Value::as_str) {
        println!("  {label}: {}", terminal_safe_text(value));
    }
}

fn terminal_safe_text(value: &str) -> std::borrow::Cow<'_, str> {
    if !value.chars().any(char::is_control) {
        return std::borrow::Cow::Borrowed(value);
    }

    std::borrow::Cow::Owned(value.chars().flat_map(char::escape_default).collect())
}

fn json_field_as_str<'a>(value: &'a serde_json::Value, field: &str) -> &'a str {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}

fn json_field_as_bool_string(value: &serde_json::Value, field: &str) -> &'static str {
    match value.get(field).and_then(serde_json::Value::as_bool) {
        Some(true) => "true",
        Some(false) => "false",
        None => "",
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
                vec!["lifecycle-plan"],
                operations::SERVICE_LIFECYCLE_PLAN,
            ),
            (
                ServiceCliFamily::Service,
                vec!["lifecycle-status"],
                operations::SERVICE_LIFECYCLE_STATUS,
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
        assert_eq!(operations.len(), 15);
        assert!(operations.contains(&operations::SERVICE_STATUS.to_string()));
        assert!(operations.contains(&operations::SERVICE_LIFECYCLE_PLAN.to_string()));
        assert!(operations.contains(&operations::SERVICE_LIFECYCLE_STATUS.to_string()));
        assert!(operations.contains(&operations::PROJECT_STATUS.to_string()));
        assert!(operations.contains(&operations::WIZARD_SESSION_DIFF.to_string()));
        assert!(operations.contains(&operations::JOBS_GET.to_string()));
        assert!(operations.contains(&operations::RECEIPTS_GET.to_string()));
    }

    #[test]
    fn default_text_renderer_escapes_terminal_control_characters() {
        let raw =
            "redb outbox journal `evil-\u{1b}]52;c;SGFja2Vk\u{7}.redb` failed\nnext-line\twith-tab";
        let escaped = terminal_safe_text(raw);

        assert_eq!(
            escaped,
            "redb outbox journal `evil-\\u{1b}]52;c;SGFja2Vk\\u{7}.redb` failed\\nnext-line\\twith-tab"
        );
        assert!(!escaped.contains('\u{1b}'));
        assert!(!escaped.contains('\u{7}'));
        assert!(!escaped.contains('\n'));
        assert!(!escaped.contains('\t'));
    }

    #[test]
    fn default_text_renderer_preserves_safe_text() {
        let raw = "Inspect outbox `outbox-1` failure `network failed`, then requeue.";
        assert!(matches!(
            terminal_safe_text(raw),
            std::borrow::Cow::Borrowed(_)
        ));
        assert_eq!(terminal_safe_text(raw), raw);
    }

    #[test]
    fn cli_service_client_rejects_unknown_family_command_before_client_execution() {
        let err = service_cli_request(ServiceCliFamily::Wizard, &[String::from("apply")])
            .expect_err("unsupported command should fail closed");
        assert!(err.contains("Unsupported service-client command"));
    }
}
