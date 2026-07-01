use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use runtime_path_policy::{
    read_bounded_text_file_under_root, ArtifactPathKind, PathPolicyError, StateRoot,
};
use serde_json::{json, Value};

use crate::config_value_utils::{
    load_project_overlay_yaml, yaml_bool, yaml_lookup, yaml_string, yaml_string_list,
};
use crate::{RequirementAnalyzeArgs, RequirementArgs, RequirementCommand};

const SURFACE: &str = "vida requirement analyze";
const SCHEMA_VERSION: &str = "requirement-analysis-artifact.v1";
const MAX_SOURCE_FILE_BYTES: u64 = 64 * 1024;

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

#[derive(Debug, Clone)]
struct RequirementSourceInput {
    kind: &'static str,
    serialized_text: String,
    source_metadata: Option<String>,
    public_analysis_text: String,
}

struct RequirementSourceRedaction {
    public_text: String,
    redacted: bool,
}

impl RequirementSourceInput {
    fn operator_text(text: String) -> Self {
        Self {
            kind: "operator_text",
            serialized_text: text.clone(),
            source_metadata: None,
            public_analysis_text: text,
        }
    }
}

fn read_requirement_source_file(path: &Path) -> Result<RequirementSourceInput, String> {
    let project_root = requirement_source_project_root()?;
    let relative_path = validate_requirement_source_path(path)?;
    reject_symlink_components(&project_root, &relative_path)?;
    let state_root =
        StateRoot::open(&project_root).map_err(|error| format!("{}: {error}", path.display()))?;
    let content = read_bounded_text_file_under_root(
        &state_root,
        &relative_path,
        ArtifactPathKind::RequirementSourceFile,
        MAX_SOURCE_FILE_BYTES,
    )
    .map_err(|error| requirement_source_path_error(path, error))?;
    let display_path = relative_path.display().to_string();
    let redaction = redact_requirement_source_content(content.trim());
    let digest = blake3::hash(redaction.public_text.as_bytes());
    let redacted_flag = if redaction.redacted {
        ":redacted=true"
    } else {
        ""
    };
    Ok(RequirementSourceInput {
        kind: "source_file",
        serialized_text: redaction.public_text.clone(),
        source_metadata: Some(format!(
            "file:{display_path}:bytes={}:blake3={digest}{redacted_flag}",
            redaction.public_text.len()
        )),
        public_analysis_text: redaction.public_text,
    })
}

fn requirement_source_path_error(path: &Path, error: PathPolicyError) -> String {
    match error {
        PathPolicyError::Symlink { .. } => {
            format!("{}: source file must not be a symlink", path.display())
        }
        PathPolicyError::NotRegularFile { .. } => {
            format!("{}: source file must be a regular file", path.display())
        }
        PathPolicyError::TooLarge { max_bytes, .. } => {
            format!(
                "{}: source file exceeds {max_bytes} byte limit",
                path.display()
            )
        }
        other => format!("{}: {other}", path.display()),
    }
}

fn requirement_source_project_root() -> Result<PathBuf, String> {
    crate::resolve_runtime_project_root()
}

fn redact_requirement_source_content(content: &str) -> RequirementSourceRedaction {
    let mut in_secret_block = false;
    let mut redacted = false;
    let mut public_lines = Vec::new();
    for line in content.lines() {
        let starts_secret_block = requirement_source_secret_assignment_opens_multiline(line);
        let secret_line = in_secret_block
            || starts_secret_block
            || requirement_source_secret_assignment_line(line)
            || line.split_whitespace().any(requirement_source_secret_token);
        if secret_line {
            redacted = true;
            public_lines.push("[redacted source-file secret line]".to_string());
            if in_secret_block && requirement_source_secret_block_closes(line) {
                in_secret_block = false;
            } else if starts_secret_block {
                in_secret_block = true;
            }
        } else {
            public_lines.push(line.to_string());
        }
    }
    RequirementSourceRedaction {
        public_text: public_lines.join("\n"),
        redacted,
    }
}

fn requirement_source_secret_assignment_line(line: &str) -> bool {
    let Some(delimiter_index) = line.find(['=', ':']) else {
        return false;
    };
    let key = &line[..delimiter_index];
    requirement_source_assignment_key_like(key) && requirement_source_secret_key(key)
}

fn requirement_source_secret_assignment_opens_multiline(line: &str) -> bool {
    let Some(delimiter_index) = line.find(['=', ':']) else {
        return false;
    };
    let key = &line[..delimiter_index];
    if !requirement_source_assignment_key_like(key) || !requirement_source_secret_key(key) {
        return false;
    }
    let value = line[delimiter_index + 1..].trim_start();
    (value.contains("-----BEGIN") && !value.contains("-----END"))
        || value
            .strip_prefix('"')
            .is_some_and(|rest| !rest.contains('"'))
        || value
            .strip_prefix('\'')
            .is_some_and(|rest| !rest.contains('\''))
}

fn requirement_source_secret_block_closes(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty()
        || trimmed.contains("-----END")
        || trimmed.ends_with('"')
        || trimmed.ends_with('\'')
}

fn requirement_source_secret_token(token: &str) -> bool {
    let Some((key, _)) = token.split_once('=') else {
        return false;
    };
    requirement_source_secret_key(key)
}

fn requirement_source_assignment_key_like(key: &str) -> bool {
    let key = key.trim().trim_matches(['"', '\'']);
    !key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
}

fn requirement_source_secret_key(key: &str) -> bool {
    let key = key
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .to_ascii_lowercase()
        .replace('-', "_");
    key.contains("secret")
        || key.contains("token")
        || key.contains("password")
        || key.contains("credential")
        || key.contains("private_key")
        || key.contains("api_key")
        || key.contains("apikey")
}

fn validate_requirement_source_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Err(format!(
            "{}: source file must be relative to the project root",
            path.display()
        ));
    }

    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(format!(
                    "{}: source file must not contain parent-directory traversal",
                    path.display()
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "{}: source file must be relative to the project root",
                    path.display()
                ));
            }
        }
    }

    if relative.as_os_str().is_empty() {
        return Err("source file path must not be empty".to_string());
    }

    Ok(relative)
}

fn reject_symlink_components(project_root: &Path, relative_path: &Path) -> Result<(), String> {
    let mut current = PathBuf::from(project_root);
    for component in relative_path.components() {
        current.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("{}: {error}", relative_path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "{}: source file path must not contain symlinks",
                relative_path.display()
            ));
        }
    }
    Ok(())
}

fn requirement_analysis_artifact(args: &RequirementAnalyzeArgs) -> Result<Value, String> {
    let mut source_inputs = args
        .input
        .iter()
        .cloned()
        .map(RequirementSourceInput::operator_text)
        .collect::<Vec<_>>();
    if let Some(path) = &args.source_file {
        source_inputs.push(read_requirement_source_file(path)?);
    }
    if source_inputs.is_empty() {
        source_inputs.push(RequirementSourceInput::operator_text(
            "operator_request_text_or_artifact_path".to_string(),
        ));
    }

    let identity = args
        .task_id
        .as_deref()
        .or(args.request_id.as_deref())
        .unwrap_or("unbound-requirement");
    let combined_input = source_inputs
        .iter()
        .map(|source| source.public_analysis_text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let atoms = requirement_atoms(&source_inputs);
    let conflicts = detected_conflicts(&combined_input);
    let party_chat_route = requirement_party_chat_route(&args, &combined_input, &conflicts);

    Ok(json!({
        "artifact_kind": "requirement_analysis",
        "schema_version": SCHEMA_VERSION,
        "surface": SURFACE,
        "task_id": args.task_id,
        "request_id": args.request_id,
        "artifact_path": args.artifact_path.as_ref().map(|path| path.display().to_string()),
        "source_inputs": source_inputs.iter().enumerate().map(|(index, input)| {
            let mut source_input = json!({
                "id": format!("source-{}", index + 1),
                "kind": input.kind,
                "text": input.serialized_text,
                "analysis_text": input.public_analysis_text,
            });
            if let Some(metadata) = &input.source_metadata {
                source_input["source_metadata"] = json!(metadata);
            }
            source_input
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
        "detected_conflicts": conflicts,
        "challenge_route": party_chat_route,
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
        "challenge_route",
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

fn requirement_atoms(source_inputs: &[RequirementSourceInput]) -> Vec<Value> {
    source_inputs
        .iter()
        .flat_map(|input| input.public_analysis_text.split(['.', ';', '\n']))
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

fn requirement_party_chat_route(
    args: &RequirementAnalyzeArgs,
    combined_input: &str,
    detected_conflicts: &[Value],
) -> Value {
    let Ok(config) = load_project_overlay_yaml() else {
        return json!({
            "recommended": false,
            "route_id": null,
            "reason": "project_config_unavailable"
        });
    };
    let Some(route_config) = yaml_lookup(&config, &["requirement_analysis", "party_chat_route"])
    else {
        return json!({
            "recommended": false,
            "route_id": null,
            "reason": "party_chat_route_not_configured"
        });
    };
    if !yaml_bool(yaml_lookup(route_config, &["enabled"]), false) {
        return json!({
            "recommended": false,
            "route_id": yaml_string(yaml_lookup(route_config, &["route_id"])),
            "reason": "party_chat_route_disabled"
        });
    }

    let trigger_matches =
        party_chat_trigger_matches(route_config, args, combined_input, detected_conflicts);
    let recommended = !trigger_matches.is_empty();

    json!({
        "recommended": recommended,
        "route_owner": yaml_string(yaml_lookup(route_config, &["route_owner"])),
        "route_id": yaml_string(yaml_lookup(route_config, &["route_id"])),
        "board_flow_id": yaml_string(yaml_lookup(route_config, &["board_flow_id"])),
        "activation_policy": yaml_string(yaml_lookup(route_config, &["activation_policy"])),
        "default_for_routine_requirements": yaml_bool(yaml_lookup(route_config, &["default_for_routine_requirements"]), false),
        "trigger_matches": trigger_matches,
        "structured_output_contract": yaml_string_list(yaml_lookup(route_config, &["structured_output_contract"])),
        "guardrails": yaml_string_list(yaml_lookup(route_config, &["guardrails"])),
        "next_action": if recommended {
            "Shape an optional Party Chat challenge-round packet through the configured board flow; TaskFlow writer, coach, verifier, approval, and closure law remain authoritative."
        } else {
            "Do not run Party Chat for this routine requirement."
        }
    })
}

fn party_chat_trigger_matches(
    route_config: &serde_yaml::Value,
    args: &RequirementAnalyzeArgs,
    combined_input: &str,
    detected_conflicts: &[Value],
) -> Vec<Value> {
    let normalized_input = combined_input.to_lowercase();
    let mut matches = Vec::new();
    let Some(serde_yaml::Value::Sequence(triggers)) = yaml_lookup(route_config, &["triggers"])
    else {
        return matches;
    };

    for trigger in triggers {
        let trigger_id = yaml_string(yaml_lookup(trigger, &["trigger_id"]))
            .unwrap_or_else(|| "unnamed_trigger".to_string());
        let kind = yaml_string(yaml_lookup(trigger, &["kind"])).unwrap_or_default();
        match kind.as_str() {
            "depth_mode" => {
                let values = yaml_string_list(yaml_lookup(trigger, &["values"]));
                if values.iter().any(|value| value == args.depth_mode.as_str()) {
                    matches.push(json!({
                        "trigger_id": trigger_id,
                        "kind": kind,
                        "matched": args.depth_mode.as_str()
                    }));
                }
            }
            "source_terms" => {
                let matched_terms = yaml_string_list(yaml_lookup(trigger, &["terms"]))
                    .into_iter()
                    .filter(|term| normalized_input.contains(&term.to_lowercase()))
                    .collect::<Vec<_>>();
                if !matched_terms.is_empty() {
                    matches.push(json!({
                        "trigger_id": trigger_id,
                        "kind": kind,
                        "matched_terms": matched_terms
                    }));
                }
            }
            "detected_conflicts" => {
                if !detected_conflicts.is_empty() {
                    matches.push(json!({
                        "trigger_id": trigger_id,
                        "kind": kind,
                        "matched": detected_conflicts.len()
                    }));
                }
            }
            _ => {}
        }
    }

    matches
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
