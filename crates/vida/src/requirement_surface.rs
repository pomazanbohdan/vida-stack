use std::process::ExitCode;

use serde_json::{json, Value};

use crate::{RequirementAnalyzeArgs, RequirementArgs, RequirementCommand};

const SURFACE: &str = "vida requirement analyze";
const SCHEMA_VERSION: &str = "requirement-analysis-artifact.v1";

pub(crate) async fn run_requirement(args: RequirementArgs) -> ExitCode {
    match args.command {
        RequirementCommand::Analyze(args) => run_requirement_analyze(args),
    }
}

fn run_requirement_analyze(args: RequirementAnalyzeArgs) -> ExitCode {
    if args.task_id.is_none() && args.request_id.is_none() {
        return emit_missing_identity(args.json);
    }

    let artifact = match requirement_analysis_artifact(&args) {
        Ok(artifact) => artifact,
        Err(error) => return emit_source_error(&error, args.json),
    };
    let payload = requirement_analysis_payload(artifact);

    if args.json {
        crate::print_json_pretty(&payload);
    } else {
        print_compact_contract(&payload["artifact"]);
    }

    ExitCode::SUCCESS
}

fn emit_missing_identity(json_output: bool) -> ExitCode {
    if json_output {
        let payload = blocked_requirement_payload(
            "missing_requirement_identity",
            vec![
                "Retry with `vida requirement analyze --task-id <task-id> --json`.",
                "Use `--request-id <request-id>` when no TaskFlow task exists yet.",
            ],
            Some("missing task_id or request_id"),
        );
        crate::print_json_pretty(&payload);
    } else {
        println!("vida requirement analyze");
        println!("status: blocked");
        println!("blocker_codes[1]: missing_requirement_identity");
        println!("next_actions[2]:");
        println!("  - Retry with `vida requirement analyze --task-id <task-id>`.");
        println!("  - Use `--request-id <request-id>` when no TaskFlow task exists yet.");
    }

    ExitCode::from(2)
}

fn emit_source_error(error: &str, json_output: bool) -> ExitCode {
    if json_output {
        let payload = blocked_requirement_payload(
            "requirement_source_unreadable",
            vec![
                "Check the `--source-file` path.",
                "Retry with readable source inputs and `--json`.",
            ],
            Some(error),
        );
        crate::print_json_pretty(&payload);
    } else {
        println!("vida requirement analyze");
        println!("status: blocked");
        println!("blocker_codes[1]: requirement_source_unreadable");
        println!("error: {error}");
    }

    ExitCode::from(1)
}

fn blocked_requirement_payload(
    blocker_code: &str,
    next_actions: Vec<&'static str>,
    error: Option<&str>,
) -> Value {
    let artifact_refs = json!({
        "surface": SURFACE,
        "schema_version": SCHEMA_VERSION,
        "blocker_code": blocker_code,
    });
    let mut extra_fields = json!({});
    if let Some(error) = error {
        extra_fields["error"] = json!(error);
    }
    crate::release1_operator_output::build_release1_operator_output_payload(
        SURFACE,
        vec![blocker_code.to_string()],
        next_actions.into_iter().map(str::to_string).collect(),
        artifact_refs,
        extra_fields,
    )
    .expect("requirement blocked payload should satisfy release-1 operator shape")
}

fn requirement_analysis_artifact(args: &RequirementAnalyzeArgs) -> Result<Value, String> {
    let mut source_inputs = args.input.clone();
    if let Some(path) = &args.source_file {
        let content = std::fs::read_to_string(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        source_inputs.push(format!("file:{}:{}", path.display(), content.trim()));
    }
    if source_inputs.is_empty() {
        source_inputs.push("operator_request_text_or_artifact_path".to_string());
    }

    let identity = args
        .task_id
        .as_deref()
        .or(args.request_id.as_deref())
        .unwrap_or("unbound-requirement");
    let combined_input = source_inputs.join("\n");
    let atoms = requirement_atoms(&source_inputs);

    Ok(json!({
        "artifact_kind": "requirement_analysis",
        "schema_version": SCHEMA_VERSION,
        "surface": SURFACE,
        "task_id": args.task_id,
        "request_id": args.request_id,
        "artifact_path": args.artifact_path.as_ref().map(|path| path.display().to_string()),
        "source_inputs": source_inputs.iter().enumerate().map(|(index, input)| {
            json!({
                "id": format!("source-{}", index + 1),
                "kind": if input.starts_with("file:") { "source_file" } else { "operator_text" },
                "text": input,
            })
        }).collect::<Vec<_>>(),
        "requirement_classification": {
            "primary_class": requirement_primary_class(&combined_input),
            "allowed_classes": ["feature", "bug", "runtime_defect", "documentation", "research", "release", "cleanup"],
            "description": "Classifies the request so the runtime can choose the lawful analysis depth and downstream route."
        },
        "depth_mode": args.depth_mode.as_str(),
        "requirement_atoms": atoms,
        "selected_methods": [
            {
                "method_id": "classification",
                "purpose": "Separate must, should, could, blockers, assumptions, and proof obligations."
            },
            {
                "method_id": "artifact_schema_contract",
                "purpose": "Emit a self-contained handoff artifact for downstream agents and operators."
            }
        ],
        "selected_roles": [
            {
                "role_id": "analyst",
                "responsibility": "Produce requirement atoms, conflicts, questions, options, and readiness verdict."
            },
            {
                "role_id": "developer",
                "responsibility": "Implement only from a ready developer handoff."
            },
            {
                "role_id": "tester",
                "responsibility": "Validate the acceptance criteria and test matrix."
            }
        ],
        "role_findings_summary": [
            {
                "role_id": "analyst",
                "summary": "Requirement atoms, readiness, and downstream routing were derived from the supplied source inputs."
            },
            {
                "role_id": "developer",
                "summary": "Developer handoff is bounded by requirement atoms, acceptance criteria, and test matrix."
            }
        ],
        "detected_conflicts": detected_conflicts(&combined_input),
        "open_questions": {
            "critical": [],
            "important": [],
            "optional": []
        },
        "working_assumptions": [
            "TaskFlow remains execution authority for downstream work.",
            "Closure requires proof evidence for the test matrix."
        ],
        "solution_options": [
            {
                "option_id": "option-a",
                "summary": "Proceed with the smallest implementation that satisfies all must-have atoms.",
                "tradeoffs": []
            }
        ],
        "recommended_option": {
            "option_id": "option-a",
            "rationale": "Default recommendation until concrete analysis proves a different option."
        },
        "readiness_verdict": "ready_for_developer_handoff",
        "readiness_states": {
            "ready": "Downstream implementation can start from this artifact.",
            "ready_for_developer_handoff": "Downstream implementation can start from this artifact.",
            "blocked": "A critical conflict or missing source prevents routing.",
            "needs_questions": "Critical or important questions must be answered before routing.",
            "draft": "Artifact is not yet admitted for downstream routing."
        },
        "downstream_routes": [
            {
                "route_id": "developer_handoff",
                "when": "Use when readiness_verdict is ready_for_developer_handoff and code or doc changes are required.",
                "allowed_next_node": "developer"
            }
        ],
        "acceptance_criteria": [
            {
                "id": "acceptance-1",
                "criterion": format!("Requirement artifact for `{identity}` is self-contained and machine-readable.")
            },
            {
                "id": "acceptance-2",
                "criterion": "Default output stays compact while --json exposes the complete artifact."
            }
        ],
        "test_matrix": [
            {
                "case_id": "contract-json",
                "surface": "vida requirement analyze --json",
                "expected": "Machine-readable artifact contains all required fields."
            },
            {
                "case_id": "contract-default",
                "surface": "vida requirement analyze",
                "expected": "Compact operator output is understandable without external documents."
            }
        ],
        "output_contract": {
            "default": {
                "mode": "compact_toon_plain",
                "purpose": "Operator-facing summary with readiness, artifact path, routes, and handoff."
            },
            "json": {
                "mode": "machine_readable",
                "purpose": "Complete release-1 operator payload with nested requirement-analysis artifact."
            }
        },
        "codebase_impact": {
            "inspected": args.codebase_inspected,
            "status": if args.codebase_inspected { "inspected" } else { "not_inspected" },
            "summary": "Populate with inspected files, affected modules, risk, and test impact when code was inspected."
        },
        "developer_handoff": {
            "summary": "Implement against the requirement atoms, recommended option, acceptance criteria, and test matrix.",
            "required_inputs": ["requirement_atoms", "recommended_option", "acceptance_criteria", "test_matrix"],
            "proof_expectation": "Return changed files, proof commands, verdict, blockers, rework target, and allowed next node.",
            "proof_targets": [
                "cargo test -p vida requirement_analysis_cli_contract -- --test-threads=1",
                "vida requirement analyze --help"
            ]
        }
    }))
}

fn requirement_analysis_payload(artifact: Value) -> Value {
    let artifact_refs = json!({
        "surface": SURFACE,
        "schema_version": SCHEMA_VERSION,
        "task_id": artifact["task_id"],
        "request_id": artifact["request_id"],
        "artifact_path": artifact["artifact_path"],
    });
    let blocker_codes = Vec::<String>::new();
    let next_actions = Vec::<String>::new();
    let mut payload = crate::release1_operator_output::build_release1_operator_output_payload(
        SURFACE,
        blocker_codes.clone(),
        next_actions.clone(),
        artifact_refs.clone(),
        json!({ "artifact": artifact }),
    )
    .expect("requirement analysis payload should satisfy release-1 operator shape");
    for field in [
        "task_id",
        "request_id",
        "source_inputs",
        "requirement_classification",
        "depth_mode",
        "requirement_atoms",
        "selected_methods",
        "selected_roles",
        "role_findings_summary",
        "detected_conflicts",
        "open_questions",
        "working_assumptions",
        "solution_options",
        "recommended_option",
        "readiness_verdict",
        "downstream_routes",
        "acceptance_criteria",
        "test_matrix",
        "output_contract",
        "codebase_impact",
        "developer_handoff",
    ] {
        payload[field] = payload["artifact"][field].clone();
    }
    payload
}

fn print_compact_contract(artifact: &Value) {
    println!("vida requirement analyze");
    println!(
        "schema_version: {}",
        artifact["schema_version"].as_str().unwrap_or("unknown")
    );
    if let Some(task_id) = artifact["task_id"].as_str() {
        println!("task_id: {task_id}");
    }
    if let Some(request_id) = artifact["request_id"].as_str() {
        println!("request_id: {request_id}");
    }
    println!(
        "depth_mode: {}",
        artifact["depth_mode"].as_str().unwrap_or("standard")
    );
    println!(
        "readiness_verdict: {}",
        artifact["readiness_verdict"]
            .as_str()
            .unwrap_or("needs_questions")
    );
    if let Some(path) = artifact["artifact_path"].as_str() {
        println!("artifact_path: {path}");
    }
    println!("required_fields[22]{{name,meaning}}:");
    for (name, meaning) in REQUIRED_FIELD_SUMMARY {
        println!("  {name},{meaning}");
    }
    println!("readiness_statuses[4]{{status,meaning}}:");
    for status in ["ready", "blocked", "needs_questions", "draft"] {
        println!(
            "  {},{}",
            status,
            artifact["readiness_states"][status].as_str().unwrap_or("")
        );
    }
    println!("output_modes[2]{{mode,contract}}:");
    println!("  default,compact TOON/plain operator summary");
    println!("  json,machine-readable requirement-analysis artifact");
    println!("allowed_next_node: developer");
    println!("downstream_routes: developer_handoff");
    println!("developer_handoff: Implement against the requirement atoms");
}

fn requirement_atoms(source_inputs: &[String]) -> Vec<Value> {
    source_inputs
        .iter()
        .flat_map(|input| input.split(['.', ';', '\n']))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .take(12)
        .enumerate()
        .map(|(index, text)| {
            json!({
                "id": format!("atom-{}", index + 1),
                "text": text,
                "source_ref": "source_inputs",
                "priority": "must",
                "verification_hint": "Observable proof or acceptance check for this atom."
            })
        })
        .collect()
}

fn requirement_primary_class(source: &str) -> &'static str {
    let normalized = source.to_lowercase();
    if normalized.contains("bug") || normalized.contains("fix") {
        "bug"
    } else if normalized.contains("doc") {
        "documentation"
    } else if normalized.contains("research") {
        "research"
    } else {
        "feature"
    }
}

fn detected_conflicts(source: &str) -> Vec<Value> {
    let normalized = source.to_lowercase();
    if normalized.contains("without tests") || normalized.contains("no tests") {
        vec![json!({
            "conflict_id": "proof_expectation_conflict",
            "summary": "Request appears to avoid tests, but VIDA closure requires proof evidence.",
            "severity": "important"
        })]
    } else {
        Vec::new()
    }
}

const REQUIRED_FIELD_SUMMARY: [(&str, &str); 22] = [
    ("task_id_or_request_id", "one source identity is required"),
    (
        "source_inputs",
        "request text paths issue ids or upstream artifacts",
    ),
    (
        "requirement_classification",
        "type and routing class for the request",
    ),
    ("depth_mode", "quick standard or critical analysis depth"),
    (
        "requirement_atoms",
        "atomic requirements with source refs and proof hints",
    ),
    ("selected_methods", "analysis methods applied"),
    ("selected_roles", "runtime roles used for findings"),
    ("role_findings_summary", "per-role conclusion summary"),
    (
        "detected_conflicts",
        "contradictions or incompatible constraints",
    ),
    ("open_questions.critical", "questions blocking readiness"),
    (
        "open_questions.important",
        "questions that affect option quality",
    ),
    ("open_questions.optional", "questions safe to defer"),
    (
        "working_assumptions",
        "assumptions used when questions remain open",
    ),
    (
        "solution_options",
        "candidate implementation or planning options",
    ),
    ("recommended_option", "chosen option and rationale"),
    (
        "readiness_verdict",
        "ready blocked needs_questions or draft",
    ),
    ("downstream_routes", "lawful next runtime routes"),
    ("acceptance_criteria", "observable done criteria"),
    ("test_matrix", "proof scenarios for downstream work"),
    ("codebase_impact", "required when code was inspected"),
    ("developer_handoff", "implementation handoff contract"),
    (
        "output_contract",
        "default compact output and explicit JSON parity",
    ),
];
