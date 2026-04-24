use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::{Deserialize, Serialize};

use crate::state_store::{
    CreateTaskRequest, StateStore, StateStoreError, TaskDependencyRecord, TaskExecutionSemantics,
    TaskGraphIssue, TaskPlannerMetadata, TaskRecord,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TaskPlanGraphDraft {
    pub draft_id: String,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub source_hash: String,
    pub task_prefix: String,
    pub parent_id: Option<String>,
    pub generated_by: String,
    pub deterministic: bool,
    #[serde(default = "default_input_contract")]
    pub input_contract: TaskPlanInputContract,
    pub nodes: Vec<TaskPlanNodeDraft>,
    pub edges: Vec<TaskPlanEdgeDraft>,
    pub validation: TaskPlanValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TaskPlanInputContract {
    pub status: String,
    pub sources: Vec<TaskPlanInputSource>,
    pub missing_context: Vec<String>,
    pub operator_truth: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TaskPlanInputSource {
    pub source_type: String,
    pub reference: String,
    pub status: String,
    pub evidence: String,
}

fn default_input_contract() -> TaskPlanInputContract {
    TaskPlanInputContract {
        status: "legacy_missing".to_string(),
        sources: Vec::new(),
        missing_context: vec![
            "input_contract_missing_from_legacy_draft".to_string(),
            "spec_reference_missing".to_string(),
            "backlog_reference_missing".to_string(),
            "context_reference_missing".to_string(),
        ],
        operator_truth: vec![
            "planner_input_contract_missing_from_legacy_draft_requires_operator_confirmation"
                .to_string(),
        ],
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TaskPlanNodeDraft {
    pub task_id: String,
    pub title: String,
    pub description: String,
    pub issue_type: String,
    pub status: String,
    pub priority: u32,
    pub parent_id: Option<String>,
    pub labels: Vec<String>,
    pub execution_semantics: TaskExecutionSemantics,
    pub owned_paths: Vec<String>,
    pub acceptance_targets: Vec<String>,
    pub proof_targets: Vec<String>,
    pub evidence_confidence: String,
    pub operator_truth: Vec<String>,
    pub risk: String,
    pub estimate: String,
    pub lane_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TaskPlanEdgeDraft {
    pub task_id: String,
    pub depends_on_id: String,
    pub edge_type: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TaskPlanValidation {
    pub status: String,
    pub blocker_codes: Vec<String>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TaskPlanMaterializationReceipt {
    pub status: String,
    pub dry_run: bool,
    pub draft_id: String,
    pub created_task_ids: Vec<String>,
    pub skipped_existing_task_ids: Vec<String>,
    pub dependency_edges: Vec<TaskPlanEdgeDraft>,
    pub validation: TaskPlanValidation,
    pub graph_validation: Vec<TaskGraphIssue>,
}

#[derive(Debug, Clone, Default)]
struct PlanGenerateOptions {
    source_file: Option<PathBuf>,
    source_text: Option<String>,
    spec_refs: Vec<String>,
    backlog_refs: Vec<String>,
    context_refs: Vec<String>,
    task_prefix: Option<String>,
    parent_id: Option<String>,
    output: Option<PathBuf>,
    json: bool,
}

#[derive(Debug, Clone)]
struct PlanMaterializeOptions {
    draft_path: PathBuf,
    state_dir: PathBuf,
    dry_run: bool,
    json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlanSourceAnalysis {
    work_items: Vec<PlanWorkItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlanWorkItemKind {
    Core,
    State,
    Runtime,
    Docs,
    Proof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlanWorkItem {
    slug_hint: String,
    title: String,
    description: String,
    labels: Vec<String>,
    kind: PlanWorkItemKind,
    owned_paths: Vec<String>,
    acceptance_targets: Vec<String>,
    proof_targets: Vec<String>,
    evidence_confidence: String,
    operator_truth: Vec<String>,
    risk: String,
    estimate: String,
    lane_hint: String,
    execution_semantics: TaskExecutionSemantics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedPathSuggestion {
    paths: Vec<String>,
    source: &'static str,
}

pub(crate) async fn run_taskflow_plan(args: &[String]) -> ExitCode {
    match args.get(1).map(String::as_str) {
        Some("generate") => match parse_generate_options(args) {
            Ok(options) => run_plan_generate(options).await,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(2)
            }
        },
        Some("materialize") => match parse_materialize_options(args) {
            Ok(options) => run_plan_materialize(options).await,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(2)
            }
        },
        _ => {
            print_plan_help();
            ExitCode::SUCCESS
        }
    }
}

fn print_plan_help() {
    println!(
        "TaskFlow PlanGraph surfaces\n\n  vida taskflow plan generate --source-file <path> --task-prefix <prefix> [--spec-ref <ref>] [--backlog-ref <ref>] [--context-ref <ref>] --json\n  vida taskflow plan generate --source-text <text> --task-prefix <prefix> [--spec-ref <ref>] [--backlog-ref <ref>] [--context-ref <ref>] --json\n  vida taskflow plan materialize <draft.json> --dry-run --json\n  vida taskflow plan materialize <draft.json> --json"
    );
}

fn parse_generate_options(args: &[String]) -> Result<PlanGenerateOptions, String> {
    let mut options = PlanGenerateOptions::default();
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--source-file" => {
                index += 1;
                options.source_file = Some(PathBuf::from(required_value(args, index, "--source-file")?));
            }
            "--source-text" => {
                index += 1;
                options.source_text = Some(required_value(args, index, "--source-text")?.to_string());
            }
            "--spec-ref" => {
                index += 1;
                options.spec_refs.push(required_value(args, index, "--spec-ref")?.to_string());
            }
            "--backlog-ref" => {
                index += 1;
                options
                    .backlog_refs
                    .push(required_value(args, index, "--backlog-ref")?.to_string());
            }
            "--context-ref" => {
                index += 1;
                options
                    .context_refs
                    .push(required_value(args, index, "--context-ref")?.to_string());
            }
            "--task-prefix" => {
                index += 1;
                options.task_prefix = Some(required_value(args, index, "--task-prefix")?.to_string());
            }
            "--parent-id" => {
                index += 1;
                options.parent_id = Some(required_value(args, index, "--parent-id")?.to_string());
            }
            "--output" => {
                index += 1;
                options.output = Some(PathBuf::from(required_value(args, index, "--output")?));
            }
            "--json" => options.json = true,
            "--help" | "-h" => return Err("usage: vida taskflow plan generate --source-file <path>|--source-text <text> --task-prefix <prefix> [--spec-ref <ref>] [--backlog-ref <ref>] [--context-ref <ref>] [--parent-id <id>] [--output <path>] [--json]".to_string()),
            other => return Err(format!("unknown plan generate argument `{other}`")),
        }
        index += 1;
    }
    if options.source_file.is_some() == options.source_text.is_some() {
        return Err(
            "use exactly one source: --source-file <path> or --source-text <text>".to_string(),
        );
    }
    if options
        .task_prefix
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        return Err("--task-prefix is required for deterministic task ids".to_string());
    }
    Ok(options)
}

fn parse_materialize_options(args: &[String]) -> Result<PlanMaterializeOptions, String> {
    let mut draft_path = None;
    let mut state_dir = None;
    let mut dry_run = false;
    let mut json = false;
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--state-dir" => {
                index += 1;
                state_dir = Some(PathBuf::from(required_value(args, index, "--state-dir")?));
            }
            "--dry-run" => dry_run = true,
            "--json" => json = true,
            "--help" | "-h" => return Err("usage: vida taskflow plan materialize <draft.json> [--state-dir <path>] [--dry-run] [--json]".to_string()),
            value if value.starts_with("--") => {
                return Err(format!("unknown plan materialize argument `{value}`"));
            }
            value => {
                if draft_path.is_some() {
                    return Err("plan materialize accepts exactly one draft path".to_string());
                }
                draft_path = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }
    Ok(PlanMaterializeOptions {
        draft_path: draft_path
            .ok_or_else(|| "plan materialize requires a draft path".to_string())?,
        state_dir: state_dir.unwrap_or_else(crate::state_store::default_state_dir),
        dry_run,
        json,
    })
}

fn required_value<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| format!("{flag} requires a value"))
}

async fn run_plan_generate(options: PlanGenerateOptions) -> ExitCode {
    match generate_plan_graph_draft(&options) {
        Ok(draft) => {
            if let Some(path) = options.output.as_deref() {
                if let Err(error) = write_json(path, &draft) {
                    eprintln!(
                        "failed to write PlanGraph draft `{}`: {error}",
                        path.display()
                    );
                    return ExitCode::from(1);
                }
            }
            print_json_or_plain(options.json, &draft, "generated PlanGraph draft")
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

async fn run_plan_materialize(options: PlanMaterializeOptions) -> ExitCode {
    let draft = match read_draft(&options.draft_path) {
        Ok(draft) => draft,
        Err(error) => {
            eprintln!(
                "failed to read PlanGraph draft `{}`: {error}",
                options.draft_path.display()
            );
            return ExitCode::from(1);
        }
    };
    match materialize_plan_graph_draft(&draft, &options).await {
        Ok(receipt) => print_json_or_plain(options.json, &receipt, "materialized PlanGraph draft"),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn generate_plan_graph_draft(options: &PlanGenerateOptions) -> Result<TaskPlanGraphDraft, String> {
    let (source_kind, source_ref, source_text) = if let Some(path) = options.source_file.as_deref()
    {
        (
            "file".to_string(),
            Some(path.display().to_string()),
            std::fs::read_to_string(path).map_err(|error| {
                format!("failed to read source file `{}`: {error}", path.display())
            })?,
        )
    } else {
        (
            "text".to_string(),
            None,
            options.source_text.clone().unwrap_or_default(),
        )
    };
    let normalized_source = normalize_source(&source_text);
    if normalized_source.is_empty() {
        return Err("plan source is empty after normalization".to_string());
    }
    let task_prefix = normalize_task_id_segment(options.task_prefix.as_deref().unwrap_or_default());
    if task_prefix.is_empty() {
        return Err("--task-prefix must contain at least one ASCII letter or digit".to_string());
    }
    let source_hash = stable_hash_hex(&normalized_source);
    let analysis = analyze_plan_source(&normalized_source);
    let input_contract = build_input_contract(
        &source_kind,
        source_ref.as_deref(),
        &normalized_source,
        options,
    );
    let nodes = build_nodes(&task_prefix, options.parent_id.clone(), &analysis);
    let edges = build_edges(&nodes, &analysis.work_items);
    let mut draft = TaskPlanGraphDraft {
        draft_id: format!("plan-{task_prefix}-{source_hash}"),
        source_kind,
        source_ref,
        source_hash,
        task_prefix,
        parent_id: options.parent_id.clone(),
        generated_by: "vida taskflow plan generate".to_string(),
        deterministic: true,
        input_contract,
        nodes,
        edges,
        validation: TaskPlanValidation {
            status: "unknown".to_string(),
            blocker_codes: Vec::new(),
            issues: Vec::new(),
        },
    };
    draft.validation = validate_generated_draft(&draft);
    Ok(draft)
}

fn build_input_contract(
    source_kind: &str,
    source_ref: Option<&str>,
    normalized_source: &str,
    options: &PlanGenerateOptions,
) -> TaskPlanInputContract {
    let mut sources = vec![TaskPlanInputSource {
        source_type: "primary_source".to_string(),
        reference: source_ref.unwrap_or(source_kind).to_string(),
        status: "provided".to_string(),
        evidence: format!("source_kind={source_kind}"),
    }];
    let repo_paths = extract_repo_paths(normalized_source);
    let mut spec_refs = repo_paths
        .iter()
        .filter(|path| path.starts_with("docs/product/spec/") || path.contains("/spec/"))
        .cloned()
        .collect::<Vec<_>>();
    push_input_refs(&mut spec_refs, &options.spec_refs);
    let mut context_refs = repo_paths
        .iter()
        .filter(|path| !spec_refs.iter().any(|spec| spec == *path))
        .cloned()
        .collect::<Vec<_>>();
    push_input_refs(&mut context_refs, &options.context_refs);
    let mut backlog_refs = extract_backlog_references(normalized_source);
    push_input_refs(&mut backlog_refs, &options.backlog_refs);

    for reference in &spec_refs {
        sources.push(TaskPlanInputSource {
            source_type: "spec_reference".to_string(),
            reference: reference.clone(),
            status: "provided".to_string(),
            evidence: input_reference_evidence(
                reference,
                &options.spec_refs,
                "cli_spec_ref",
                "source_text_repo_path",
            ),
        });
    }
    for reference in &context_refs {
        sources.push(TaskPlanInputSource {
            source_type: "context_reference".to_string(),
            reference: reference.clone(),
            status: "provided".to_string(),
            evidence: input_reference_evidence(
                reference,
                &options.context_refs,
                "cli_context_ref",
                "source_text_repo_path",
            ),
        });
    }
    for reference in &backlog_refs {
        sources.push(TaskPlanInputSource {
            source_type: "backlog_reference".to_string(),
            reference: reference.clone(),
            status: "provided".to_string(),
            evidence: input_reference_evidence(
                reference,
                &options.backlog_refs,
                "cli_backlog_ref",
                "source_text_task_reference",
            ),
        });
    }

    let mut missing_context = Vec::new();
    if spec_refs.is_empty() {
        missing_context.push("spec_reference_missing".to_string());
    }
    if backlog_refs.is_empty() {
        missing_context.push("backlog_reference_missing".to_string());
    }
    if context_refs.is_empty() {
        missing_context.push("context_reference_missing".to_string());
    }
    let status = if missing_context.is_empty() {
        "complete".to_string()
    } else {
        "partial".to_string()
    };
    let mut operator_truth = vec![
        "planner_input_contract_foundation_only".to_string(),
        "plan_generation_must_not_treat_missing_context_as_production_grade_evidence".to_string(),
    ];
    operator_truth.extend(
        missing_context
            .iter()
            .map(|missing| format!("{missing}_requires_operator_confirmation")),
    );
    TaskPlanInputContract {
        status,
        sources,
        missing_context,
        operator_truth,
    }
}

fn push_input_refs(refs: &mut Vec<String>, cli_refs: &[String]) {
    refs.extend(
        cli_refs
            .iter()
            .map(|reference| reference.trim())
            .filter(|reference| !reference.is_empty())
            .map(ToString::to_string),
    );
    refs.sort();
    refs.dedup();
}

fn input_reference_evidence(
    reference: &str,
    cli_refs: &[String],
    cli_evidence: &str,
    source_text_evidence: &str,
) -> String {
    if cli_refs.iter().any(|cli_ref| cli_ref.trim() == reference) {
        cli_evidence.to_string()
    } else {
        source_text_evidence.to_string()
    }
}

fn extract_backlog_references(normalized_source: &str) -> Vec<String> {
    let mut refs = normalized_source
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.'))
        .map(|token| {
            token.trim_matches(|ch: char| ch == '.' || ch == ',' || ch == ';' || ch == ':')
        })
        .filter(|token| {
            token.starts_with("vida-")
                || token.starts_with("audit-")
                || token.starts_with("feature-")
                || token.starts_with("task-")
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    refs.sort();
    refs.dedup();
    refs
}

fn analyze_plan_source(normalized_source: &str) -> PlanSourceAnalysis {
    let title = first_meaningful_line(normalized_source)
        .unwrap_or("Implement bounded project work")
        .to_string();
    let summary = summarize_source(normalized_source);
    let repo_paths = extract_repo_paths(normalized_source);
    let explicit_items = extract_preferred_work_item_texts(normalized_source);
    let work_items = if explicit_items.len() >= 2 {
        explicit_items
            .iter()
            .map(|item| build_explicit_work_item(item, &title, &summary, &repo_paths))
            .collect::<Vec<_>>()
    } else {
        build_semantic_work_items(&title, &summary, normalized_source, &repo_paths)
    };
    PlanSourceAnalysis { work_items }
}

fn build_nodes(
    task_prefix: &str,
    parent_id: Option<String>,
    analysis: &PlanSourceAnalysis,
) -> Vec<TaskPlanNodeDraft> {
    let mut slug_counts = BTreeMap::<String, usize>::new();
    analysis
        .work_items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let mut slug = normalize_task_id_segment(&item.slug_hint);
            if slug.is_empty() {
                slug = format!("work-item-{}", index + 1);
            }
            let ordinal = slug_counts.entry(slug.clone()).or_default();
            *ordinal += 1;
            let slug = if *ordinal > 1 {
                format!("{slug}-{}", ordinal)
            } else {
                slug
            };
            TaskPlanNodeDraft {
                task_id: format!("{task_prefix}-{slug}"),
                title: item.title.clone(),
                description: item.description.clone(),
                issue_type: "delivery_task".to_string(),
                status: "open".to_string(),
                priority: (index + 1) as u32,
                parent_id: parent_id.clone(),
                labels: item.labels.clone(),
                execution_semantics: item.execution_semantics.clone(),
                owned_paths: item.owned_paths.clone(),
                acceptance_targets: item.acceptance_targets.clone(),
                proof_targets: item.proof_targets.clone(),
                evidence_confidence: item.evidence_confidence.clone(),
                operator_truth: item.operator_truth.clone(),
                risk: item.risk.clone(),
                estimate: item.estimate.clone(),
                lane_hint: item.lane_hint.clone(),
            }
        })
        .collect()
}

fn build_edges(nodes: &[TaskPlanNodeDraft], work_items: &[PlanWorkItem]) -> Vec<TaskPlanEdgeDraft> {
    let mut edges = Vec::new();
    let mut last_primary_index = None;
    let mut last_docs_or_proof_index = None;
    for (index, item) in work_items.iter().enumerate() {
        let depends_on_index = match item.kind {
            PlanWorkItemKind::Docs | PlanWorkItemKind::Proof => last_docs_or_proof_index
                .or(last_primary_index)
                .or(index.checked_sub(1)),
            PlanWorkItemKind::Core | PlanWorkItemKind::State | PlanWorkItemKind::Runtime => {
                index.checked_sub(1)
            }
        };
        if let Some(depends_on_index) = depends_on_index {
            edges.push(TaskPlanEdgeDraft {
                task_id: nodes[index].task_id.clone(),
                depends_on_id: nodes[depends_on_index].task_id.clone(),
                edge_type: "blocks".to_string(),
                reason: dependency_reason_for_kind(&item.kind).to_string(),
            });
        }
        match item.kind {
            PlanWorkItemKind::Docs | PlanWorkItemKind::Proof => {
                last_docs_or_proof_index = Some(index);
            }
            PlanWorkItemKind::Core | PlanWorkItemKind::State | PlanWorkItemKind::Runtime => {
                last_primary_index = Some(index);
            }
        }
    }
    edges
}

fn dependency_reason_for_kind(kind: &PlanWorkItemKind) -> &'static str {
    match kind {
        PlanWorkItemKind::Core | PlanWorkItemKind::State | PlanWorkItemKind::Runtime => {
            "deterministic bounded implementation dependency"
        }
        PlanWorkItemKind::Docs => {
            "documentation and artifact alignment follows implementation surfaces"
        }
        PlanWorkItemKind::Proof => "proof and validation follow bounded implementation work",
    }
}

fn build_explicit_work_item(
    item_text: &str,
    source_title: &str,
    source_summary: &str,
    repo_paths: &[String],
) -> PlanWorkItem {
    let cleaned = clean_work_item_text(item_text);
    let kind = classify_work_item(&cleaned);
    let local_paths = extract_repo_paths(item_text);
    let owned_path_suggestion = suggest_owned_paths(&cleaned, &local_paths, repo_paths, &kind);
    let owned_paths = owned_path_suggestion.paths;
    let labels = labels_for_kind(&kind);
    let lane_hint = lane_hint_for_kind(&kind).to_string();
    let title = title_for_explicit_item(&cleaned, &kind);
    let description = format!(
        "Source-derived bounded work item for `{source_title}`: {}. Source summary: {source_summary}",
        compact_text(&cleaned, 220)
    );
    let acceptance_targets = acceptance_targets_for_item(&cleaned, &kind, &owned_paths);
    let proof_targets = proof_targets_for_item(&kind, &owned_paths);
    let (evidence_confidence, operator_truth) =
        planner_confidence_markers(owned_path_suggestion.source, true);
    let risk = risk_for_kind(&kind).to_string();
    let estimate = estimate_for_kind(&kind).to_string();
    let execution_semantics = execution_semantics_for_item(&kind, &cleaned);
    PlanWorkItem {
        slug_hint: cleaned.clone(),
        title,
        description,
        labels,
        kind,
        owned_paths,
        acceptance_targets,
        proof_targets,
        evidence_confidence,
        operator_truth,
        risk,
        estimate,
        lane_hint,
        execution_semantics,
    }
}

fn build_semantic_work_items(
    source_title: &str,
    source_summary: &str,
    normalized_source: &str,
    repo_paths: &[String],
) -> Vec<PlanWorkItem> {
    let source_lower = normalized_source.to_ascii_lowercase();
    let mut items = Vec::new();
    items.push(build_semantic_work_item(
        format!("Implement core scope for {source_title}"),
        format!(
            "Build the primary bounded implementation scope for `{source_title}` from source summary: {source_summary}"
        ),
        PlanWorkItemKind::Core,
        repo_paths,
    ));
    if contains_any(
        &source_lower,
        &[
            "state",
            "schema",
            "metadata",
            "model",
            "store",
            "persistence",
        ],
    ) {
        items.push(build_semantic_work_item(
            format!("Wire task state and schema surfaces for {source_title}"),
            format!(
                "Persist and align task/state model changes required by `{source_title}` using the source summary: {source_summary}"
            ),
            PlanWorkItemKind::State,
            repo_paths,
        ));
    }
    if contains_any(
        &source_lower,
        &[
            "dispatch",
            "scheduler",
            "parallel",
            "routing",
            "selection",
            "config",
            "runtime",
            "replan",
            "split",
            "spawn",
        ],
    ) {
        items.push(build_semantic_work_item(
            format!("Wire runtime and orchestration surfaces for {source_title}"),
            format!(
                "Align runtime, routing, or orchestration behavior for `{source_title}` from source summary: {source_summary}"
            ),
            PlanWorkItemKind::Runtime,
            repo_paths,
        ));
    }
    if contains_any(
        &source_lower,
        &[
            "doc",
            "spec",
            "artifact",
            "handoff",
            "preparation",
            "report",
        ],
    ) {
        items.push(build_semantic_work_item(
            format!("Align docs and artifact surfaces for {source_title}"),
            "Bring docs, reports, or artifact surfaces into parity with the bounded implementation scope."
                .to_string(),
            PlanWorkItemKind::Docs,
            repo_paths,
        ));
    }
    items.push(build_semantic_work_item(
        format!("Prove {source_title}"),
        format!(
            "Validate bounded graph safety, operator output, and proof targets for `{source_title}`."
        ),
        PlanWorkItemKind::Proof,
        repo_paths,
    ));
    items
}

fn build_semantic_work_item(
    title: String,
    description: String,
    kind: PlanWorkItemKind,
    repo_paths: &[String],
) -> PlanWorkItem {
    let owned_path_suggestion = suggest_owned_paths(&title, &[], repo_paths, &kind);
    let owned_paths = owned_path_suggestion.paths;
    let (evidence_confidence, operator_truth) =
        planner_confidence_markers(owned_path_suggestion.source, true);
    PlanWorkItem {
        slug_hint: title.clone(),
        labels: labels_for_kind(&kind),
        acceptance_targets: acceptance_targets_for_item(&title, &kind, &owned_paths),
        proof_targets: proof_targets_for_item(&kind, &owned_paths),
        evidence_confidence,
        operator_truth,
        risk: risk_for_kind(&kind).to_string(),
        estimate: estimate_for_kind(&kind).to_string(),
        lane_hint: lane_hint_for_kind(&kind).to_string(),
        execution_semantics: execution_semantics_for_item(&kind, &title),
        title,
        description,
        kind,
        owned_paths,
    }
}

fn extract_preferred_work_item_texts(normalized_source: &str) -> Vec<String> {
    let lines = normalized_source.lines().collect::<Vec<_>>();
    let lowercase_lines = lines
        .iter()
        .map(|line| line.to_ascii_lowercase())
        .collect::<Vec<_>>();
    for (index, line) in lowercase_lines.iter().enumerate() {
        if contains_any(
            line,
            &[
                "bounded items",
                "recommended split",
                "backlog split",
                "minimal fix waves",
                "fix waves",
                "recommended backlog",
            ],
        ) {
            let items = collect_list_block(&lines, index + 1);
            if items.len() >= 2 {
                return items;
            }
        }
    }
    let numbered_items = collect_list_block(&lines, 0);
    if (2..=6).contains(&numbered_items.len()) {
        return numbered_items;
    }
    Vec::new()
}

fn collect_list_block(lines: &[&str], start_index: usize) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut inside_list = false;
    for line in lines.iter().skip(start_index) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if inside_list && !current.is_empty() {
                items.push(clean_work_item_text(&current));
                current.clear();
            } else if inside_list && !items.is_empty() {
                break;
            }
            continue;
        }
        if let Some(item_body) = strip_list_marker(trimmed) {
            if !current.is_empty() {
                items.push(clean_work_item_text(&current));
                current.clear();
            }
            current.push_str(item_body);
            inside_list = true;
            continue;
        }
        if !inside_list {
            continue;
        }
        if trimmed.starts_with('#') {
            break;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(trimmed);
    }
    if !current.is_empty() {
        items.push(clean_work_item_text(&current));
    }
    items
        .into_iter()
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
}

fn strip_list_marker(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            return Some(rest.trim());
        }
    }
    let bytes = trimmed.as_bytes();
    let mut index = 0;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    if index > 0
        && index + 1 < bytes.len()
        && matches!(bytes[index] as char, '.' | ')')
        && bytes[index + 1] == b' '
    {
        return Some(trimmed[index + 2..].trim());
    }
    None
}

fn clean_work_item_text(value: &str) -> String {
    compact_text(
        &value
            .replace("**", "")
            .replace('`', "")
            .trim_matches(|ch: char| matches!(ch, '-' | '*' | '+' | ':' | ';'))
            .trim()
            .to_string(),
        220,
    )
}

fn compact_text(value: &str, max_chars: usize) -> String {
    let mut compact = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .trim_end_matches(|ch: char| matches!(ch, '.' | ',' | ';' | ':'))
        .to_string();
    if compact.chars().count() > max_chars {
        compact = compact
            .chars()
            .take(max_chars.saturating_sub(3))
            .collect::<String>();
        compact.push_str("...");
    }
    compact
}

fn classify_work_item(value: &str) -> PlanWorkItemKind {
    let lower = value.to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "proof",
            "prove",
            "test",
            "validate",
            "verification",
            "smoke",
        ],
    ) {
        PlanWorkItemKind::Proof
    } else if contains_any(
        &lower,
        &["doc", "spec", "artifact", "handoff", "report", "parity"],
    ) {
        PlanWorkItemKind::Docs
    } else if contains_any(
        &lower,
        &[
            "state",
            "schema",
            "metadata",
            "model",
            "persist",
            "queryable",
        ],
    ) {
        PlanWorkItemKind::State
    } else if contains_any(
        &lower,
        &[
            "dispatch",
            "scheduler",
            "parallel",
            "routing",
            "selection",
            "budget",
            "config",
            "readiness",
            "replan",
            "split",
            "spawn",
        ],
    ) {
        PlanWorkItemKind::Runtime
    } else {
        PlanWorkItemKind::Core
    }
}

fn title_for_explicit_item(value: &str, kind: &PlanWorkItemKind) -> String {
    let title = compact_text(value, 96);
    let lower = title.to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "implement",
            "replace",
            "persist",
            "add",
            "wire",
            "align",
            "actuate",
            "prove",
            "validate",
            "audit",
            "split",
            "spawn",
        ],
    ) {
        uppercase_first(&title)
    } else {
        match kind {
            PlanWorkItemKind::Proof => format!("Prove {}", lowercase_first(&title)),
            _ => format!("Implement {}", lowercase_first(&title)),
        }
    }
}

fn uppercase_first(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    format!("{}{}", first.to_uppercase(), chars.collect::<String>())
}

fn lowercase_first(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    format!("{}{}", first.to_lowercase(), chars.collect::<String>())
}

fn labels_for_kind(kind: &PlanWorkItemKind) -> Vec<String> {
    let mut labels = vec!["taskflow".to_string(), "plan-graph".to_string()];
    labels.push(
        match kind {
            PlanWorkItemKind::Core => "planner-core",
            PlanWorkItemKind::State => "planner-state",
            PlanWorkItemKind::Runtime => "planner-runtime",
            PlanWorkItemKind::Docs => "planner-docs",
            PlanWorkItemKind::Proof => "planner-proof",
        }
        .to_string(),
    );
    labels
}

fn lane_hint_for_kind(kind: &PlanWorkItemKind) -> &'static str {
    match kind {
        PlanWorkItemKind::Proof => "verifier",
        PlanWorkItemKind::Docs => "coach",
        PlanWorkItemKind::Core | PlanWorkItemKind::State | PlanWorkItemKind::Runtime => {
            "implementer"
        }
    }
}

fn risk_for_kind(kind: &PlanWorkItemKind) -> &'static str {
    match kind {
        PlanWorkItemKind::Core => "medium",
        PlanWorkItemKind::State | PlanWorkItemKind::Runtime => "high",
        PlanWorkItemKind::Docs | PlanWorkItemKind::Proof => "medium",
    }
}

fn estimate_for_kind(kind: &PlanWorkItemKind) -> &'static str {
    match kind {
        PlanWorkItemKind::Core => "bounded implementation packet",
        PlanWorkItemKind::State => "bounded state/schema packet",
        PlanWorkItemKind::Runtime => "bounded runtime/orchestration packet",
        PlanWorkItemKind::Docs => "bounded docs/artifact packet",
        PlanWorkItemKind::Proof => "bounded verification packet",
    }
}

fn execution_semantics_for_item(kind: &PlanWorkItemKind, value: &str) -> TaskExecutionSemantics {
    let slug = normalize_task_id_segment(value);
    let order_bucket = match kind {
        PlanWorkItemKind::Core => "plan-wave-core",
        PlanWorkItemKind::State => "plan-wave-state",
        PlanWorkItemKind::Runtime => "plan-wave-runtime",
        PlanWorkItemKind::Docs => "plan-wave-docs",
        PlanWorkItemKind::Proof => "plan-wave-proof",
    }
    .to_string();
    let conflict_prefix = match kind {
        PlanWorkItemKind::Core => "planner-core",
        PlanWorkItemKind::State => "planner-state",
        PlanWorkItemKind::Runtime => "planner-runtime",
        PlanWorkItemKind::Docs => "planner-docs",
        PlanWorkItemKind::Proof => "planner-proof",
    };
    TaskExecutionSemantics {
        execution_mode: Some("sequential".to_string()),
        order_bucket: Some(order_bucket),
        parallel_group: None,
        conflict_domain: Some(format!("{conflict_prefix}-{slug}")),
    }
}

fn acceptance_targets_for_item(
    title: &str,
    kind: &PlanWorkItemKind,
    owned_paths: &[String],
) -> Vec<String> {
    let mut targets = match kind {
        PlanWorkItemKind::Core => vec![
            format!("PlanGraph draft decomposes `{title}` into bounded project-specific task nodes instead of template placeholders"),
            "generated task nodes carry deterministic ids, labels, execution semantics, and graph-safe dependency shape".to_string(),
        ],
        PlanWorkItemKind::State => vec![
            format!("task/state surfaces required by `{title}` are wired into deterministic PlanGraph output"),
            "generated nodes remain compatible with task materialization and graph validation".to_string(),
        ],
        PlanWorkItemKind::Runtime => vec![
            format!("runtime and orchestration surfaces for `{title}` are represented by explicit bounded nodes and dependency edges"),
            "generated plan keeps scheduling and dispatch work explicit instead of collapsing it into one generic task".to_string(),
        ],
        PlanWorkItemKind::Docs => vec![
            format!("documentation or artifact alignment work for `{title}` is explicit in the plan graph"),
            "bounded docs/artifact work stays queryable as its own planned node".to_string(),
        ],
        PlanWorkItemKind::Proof => vec![
            format!("proof and validation work for `{title}` is explicit and sequenced after implementation work"),
            "generated plan preserves graph-safe proof coverage targets".to_string(),
        ],
    };
    if !owned_paths.is_empty() {
        targets.push(format!(
            "owned paths stay project-aware and source-derived: {}",
            owned_paths.join(", ")
        ));
    }
    targets
}

fn proof_targets_for_item(kind: &PlanWorkItemKind, owned_paths: &[String]) -> Vec<String> {
    let mut targets = vec!["vida taskflow plan generate --json".to_string()];
    match kind {
        PlanWorkItemKind::Docs => {
            if owned_paths.iter().any(|path| path.starts_with("docs/")) {
                targets.push(
                    "vida docflow check --root . docs/product/spec/current-spec-map.md".to_string(),
                );
            } else {
                targets.push("vida taskflow plan materialize --dry-run --json".to_string());
            }
        }
        PlanWorkItemKind::Proof => {
            targets.push("cargo test -p vida taskflow_plan_graph".to_string());
            targets.push("vida task validate-graph --json".to_string());
        }
        PlanWorkItemKind::Core | PlanWorkItemKind::State | PlanWorkItemKind::Runtime => {
            targets.push("vida taskflow plan materialize --dry-run --json".to_string());
        }
    }
    targets
}

fn planner_confidence_markers(
    owned_paths_source: &str,
    proof_targets_template_generated: bool,
) -> (String, Vec<String>) {
    let mut markers = vec![format!("owned_paths_source={owned_paths_source}")];
    if owned_paths_source == "template_fallback" {
        markers.push(
            "low_confidence_owned_paths_template_fallback_requires_operator_confirmation"
                .to_string(),
        );
    }
    if proof_targets_template_generated {
        markers.push(
            "low_confidence_proof_targets_template_guess_requires_operator_confirmation"
                .to_string(),
        );
    }
    let confidence = if markers
        .iter()
        .any(|marker| marker.starts_with("low_confidence_"))
    {
        "low"
    } else {
        "source_derived"
    };
    (confidence.to_string(), markers)
}

fn suggest_owned_paths(
    item_text: &str,
    local_paths: &[String],
    repo_paths: &[String],
    kind: &PlanWorkItemKind,
) -> OwnedPathSuggestion {
    if !local_paths.is_empty() {
        return OwnedPathSuggestion {
            paths: local_paths.to_vec(),
            source: "explicit_item_paths",
        };
    }
    let item_tokens = keyword_tokens(item_text);
    let mut scored = repo_paths
        .iter()
        .map(|path| {
            let path_lower = path.to_ascii_lowercase();
            let path_tokens = keyword_tokens(&path_lower);
            let mut score = item_tokens
                .iter()
                .filter(|token| {
                    path_tokens.iter().any(|path_token| path_token == *token)
                        || path_lower.contains(token.as_str())
                })
                .count() as i32;
            if path_lower.ends_with(".rs") {
                score += 1;
            }
            if path_lower.starts_with("docs/") {
                score += match kind {
                    PlanWorkItemKind::Docs | PlanWorkItemKind::Proof => 3,
                    _ => 0,
                };
            }
            score += match kind {
                PlanWorkItemKind::Core if contains_any(&path_lower, &["plan", "graph"]) => 6,
                PlanWorkItemKind::State
                    if contains_any(
                        &path_lower,
                        &["state_store", "task_models", "task_store", "snapshot"],
                    ) =>
                {
                    6
                }
                PlanWorkItemKind::Runtime
                    if contains_any(
                        &path_lower,
                        &[
                            "routing",
                            "dispatch",
                            "runtime_assignment",
                            "consume_bundle",
                            "proxy",
                            "graph",
                            "scheduler",
                            "carrier",
                            "profile",
                        ],
                    ) =>
                {
                    6
                }
                PlanWorkItemKind::Docs
                    if contains_any(
                        &path_lower,
                        &[
                            "docs/",
                            "spec",
                            "runtime_dispatch_state",
                            "taskflow_consume",
                        ],
                    ) =>
                {
                    5
                }
                PlanWorkItemKind::Proof
                    if contains_any(&path_lower, &["tests", "validate", "graph", "taskflow"]) =>
                {
                    5
                }
                _ => 0,
            };
            (score, path)
        })
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();
    if matches!(kind, PlanWorkItemKind::Proof) {
        scored.retain(|(_, path)| contains_any(path, &["tests", "validate"]));
    }
    scored.sort_by(|(left_score, left_path), (right_score, right_path)| {
        right_score
            .cmp(left_score)
            .then_with(|| left_path.cmp(right_path))
    });
    let mut selected = scored
        .into_iter()
        .map(|(_, path)| path.clone())
        .take(4)
        .collect::<Vec<_>>();
    let mut source = "source_repo_paths";
    if selected.is_empty() {
        selected = match kind {
            PlanWorkItemKind::Core => vec!["crates/vida/src/taskflow_plan_graph.rs".to_string()],
            PlanWorkItemKind::State => vec![
                "crates/vida/src/state_store_task_models.rs".to_string(),
                "crates/vida/src/state_store_task_store.rs".to_string(),
            ],
            PlanWorkItemKind::Runtime => vec![
                "crates/vida/src/taskflow_proxy.rs".to_string(),
                "crates/vida/src/state_store_task_graph.rs".to_string(),
            ],
            PlanWorkItemKind::Docs => vec![
                "docs/product/spec/task-graph-adaptive-planner-design.md".to_string(),
                "docs/product/spec/current-spec-map.md".to_string(),
            ],
            PlanWorkItemKind::Proof => vec!["crates/vida/tests".to_string()],
        };
        source = "template_fallback";
    }
    OwnedPathSuggestion {
        paths: selected,
        source,
    }
}

fn keyword_tokens(value: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "and", "the", "for", "with", "from", "into", "that", "this", "task", "items", "item",
        "work", "wave", "waves", "report", "final", "new", "real",
    ];
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .filter(|token| !STOPWORDS.iter().any(|stop| stop == token))
        .collect::<Vec<_>>()
}

fn extract_repo_paths(value: &str) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for token in value.split_whitespace() {
        let trimmed = token.trim_matches(|ch: char| {
            matches!(
                ch,
                '`' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | '"' | '\''
            )
        });
        if let Some(path) = canonicalize_repo_path_candidate(trimmed) {
            paths.insert(path);
        }
    }
    paths.into_iter().collect()
}

fn canonicalize_repo_path_candidate(value: &str) -> Option<String> {
    let mut candidate = value
        .trim()
        .trim_end_matches(|ch: char| matches!(ch, '.' | ',' | ';' | ':'))
        .to_string();
    if let Some((path, suffix)) = candidate.rsplit_once(':') {
        if !path.is_empty()
            && !suffix.is_empty()
            && suffix.chars().all(|ch| ch.is_ascii_digit() || ch == '-')
        {
            candidate = path.to_string();
        }
    }
    if !looks_like_repo_path(&candidate) {
        return None;
    }
    if candidate.starts_with("./") {
        candidate = candidate.trim_start_matches("./").to_string();
    }
    if candidate == "crates/vida/tests" {
        return Some(candidate);
    }
    if path_exists_from_repo_context(&candidate) || candidate == "vida.config.yaml" {
        Some(candidate)
    } else {
        None
    }
}

fn path_exists_from_repo_context(candidate: &str) -> bool {
    if Path::new(candidate).exists() {
        return true;
    }
    let Ok(current_dir) = std::env::current_dir() else {
        return false;
    };
    current_dir
        .ancestors()
        .any(|ancestor| ancestor.join(candidate).exists())
}

fn looks_like_repo_path(value: &str) -> bool {
    value.contains('/')
        || matches!(
            value,
            "vida.config.yaml" | "Cargo.toml" | "Cargo.lock" | "AGENTS.md" | "AGENTS.sidecar.md"
        )
        || value.ends_with(".rs")
        || value.ends_with(".md")
        || value.ends_with(".yaml")
        || value.ends_with(".yml")
        || value.ends_with(".toml")
        || value.ends_with(".json")
        || value.ends_with(".jsonl")
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

async fn materialize_plan_graph_draft(
    draft: &TaskPlanGraphDraft,
    options: &PlanMaterializeOptions,
) -> Result<TaskPlanMaterializationReceipt, String> {
    let store = if options.state_dir.exists() {
        StateStore::open_existing(options.state_dir.clone()).await
    } else {
        StateStore::open(options.state_dir.clone()).await
    }
    .map_err(|error| format!("failed to open task store: {error}"))?;
    let existing = store
        .list_tasks(None, true)
        .await
        .map_err(|error| format!("failed to list existing tasks: {error}"))?;
    let mut validation = validate_draft(draft, &existing);
    if validation.status != "valid" {
        return Ok(TaskPlanMaterializationReceipt {
            status: "blocked".to_string(),
            dry_run: options.dry_run,
            draft_id: draft.draft_id.clone(),
            created_task_ids: Vec::new(),
            skipped_existing_task_ids: Vec::new(),
            dependency_edges: Vec::new(),
            validation,
            graph_validation: Vec::new(),
        });
    }
    let graph_validation = simulate_materialized_graph(draft, &existing);
    if !graph_validation.is_empty() {
        validation.status = "blocked".to_string();
        validation
            .blocker_codes
            .push("graph_validation_failed".to_string());
        validation.blocker_codes.sort();
        validation.blocker_codes.dedup();
        validation
            .issues
            .extend(graph_validation.iter().map(|issue| {
                format!(
                    "graph validation issue `{}` on `{}`: {}",
                    issue.issue_type, issue.issue_id, issue.detail
                )
            }));
        return Ok(TaskPlanMaterializationReceipt {
            status: "blocked".to_string(),
            dry_run: options.dry_run,
            draft_id: draft.draft_id.clone(),
            created_task_ids: Vec::new(),
            skipped_existing_task_ids: Vec::new(),
            dependency_edges: Vec::new(),
            validation,
            graph_validation,
        });
    }
    let existing_by_id = existing
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect::<BTreeMap<_, _>>();
    let existing_ids = existing_by_id.keys().copied().collect::<BTreeSet<_>>();
    let planned_created_task_ids = draft
        .nodes
        .iter()
        .filter(|node| !existing_ids.contains(node.task_id.as_str()))
        .map(|node| node.task_id.clone())
        .collect::<Vec<_>>();
    let planned_skipped_task_ids = draft
        .nodes
        .iter()
        .filter(|node| existing_ids.contains(node.task_id.as_str()))
        .map(|node| node.task_id.clone())
        .collect::<Vec<_>>();
    if options.dry_run {
        return Ok(TaskPlanMaterializationReceipt {
            status: "dry_run".to_string(),
            dry_run: true,
            draft_id: draft.draft_id.clone(),
            created_task_ids: planned_created_task_ids,
            skipped_existing_task_ids: planned_skipped_task_ids,
            dependency_edges: draft.edges.clone(),
            validation,
            graph_validation,
        });
    }

    let mut created_task_ids = Vec::new();
    let mut skipped_existing_task_ids = Vec::new();
    for node in &draft.nodes {
        if existing_ids.contains(node.task_id.as_str()) {
            skipped_existing_task_ids.push(node.task_id.clone());
            continue;
        }
        let source_repo = std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "vida-stack".to_string());
        store
            .create_task(CreateTaskRequest {
                task_id: &node.task_id,
                title: &node.title,
                display_id: None,
                description: &node.description,
                issue_type: &node.issue_type,
                status: &node.status,
                priority: node.priority,
                parent_id: node.parent_id.as_deref(),
                labels: &node.labels,
                execution_semantics: node.execution_semantics.clone(),
                planner_metadata: planner_metadata_from_node(node),
                created_by: "vida taskflow plan materialize",
                source_repo: &source_repo,
            })
            .await
            .map_err(|error| format!("failed to create task `{}`: {error}", node.task_id))?;
        created_task_ids.push(node.task_id.clone());
    }

    let mut dependency_edges = Vec::new();
    for edge in &draft.edges {
        match store
            .add_task_dependency(
                &edge.task_id,
                &edge.depends_on_id,
                &edge.edge_type,
                "vida taskflow plan materialize",
            )
            .await
        {
            Ok(_) => dependency_edges.push(edge.clone()),
            Err(StateStoreError::InvalidTaskRecord { reason })
                if reason.contains("dependency already exists") =>
            {
                dependency_edges.push(edge.clone());
            }
            Err(error) => {
                return Err(format!(
                    "failed to add dependency `{}` -> `{}`: {error}",
                    edge.task_id, edge.depends_on_id
                ));
            }
        }
    }
    let graph_validation = store
        .validate_task_graph()
        .await
        .map_err(|error| format!("failed to validate materialized graph: {error}"))?;
    store
        .refresh_task_snapshot()
        .await
        .map_err(|error| format!("failed to refresh task snapshot: {error}"))?;
    Ok(TaskPlanMaterializationReceipt {
        status: if graph_validation.is_empty() {
            "materialized".to_string()
        } else {
            "materialized_with_graph_issues".to_string()
        },
        dry_run: false,
        draft_id: draft.draft_id.clone(),
        created_task_ids,
        skipped_existing_task_ids,
        dependency_edges,
        validation,
        graph_validation,
    })
}

fn validate_generated_draft(draft: &TaskPlanGraphDraft) -> TaskPlanValidation {
    validate_draft_inner(draft, &[], true)
}

fn validate_draft(draft: &TaskPlanGraphDraft, existing: &[TaskRecord]) -> TaskPlanValidation {
    validate_draft_inner(draft, existing, false)
}

fn validate_draft_inner(
    draft: &TaskPlanGraphDraft,
    existing: &[TaskRecord],
    allow_unresolved_external_parents: bool,
) -> TaskPlanValidation {
    let mut issues = Vec::new();
    let mut blocker_codes = Vec::new();
    let mut node_ids = BTreeSet::<&str>::new();
    let mut duplicate_ids = BTreeSet::<&str>::new();
    for node in &draft.nodes {
        if !node_ids.insert(node.task_id.as_str()) {
            duplicate_ids.insert(node.task_id.as_str());
        }
    }
    let existing_by_id = existing
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect::<BTreeMap<_, _>>();
    let existing_ids = existing_by_id.keys().copied().collect::<BTreeSet<_>>();
    for node in &draft.nodes {
        if node.task_id.trim().is_empty() {
            issues.push("node has empty task_id".to_string());
        }
        if duplicate_ids.contains(node.task_id.as_str()) {
            blocker_codes.push("duplicate_task_id".to_string());
            issues.push(format!("node `{}` is duplicated", node.task_id));
        }
        if node.title.trim().is_empty() {
            blocker_codes.push("missing_title".to_string());
            issues.push(format!("node `{}` has no title", node.task_id));
        }
        if node.owned_paths.is_empty() {
            blocker_codes.push("missing_owned_paths".to_string());
            issues.push(format!("node `{}` has no owned paths", node.task_id));
        }
        if node.acceptance_targets.is_empty() {
            blocker_codes.push("missing_acceptance_targets".to_string());
            issues.push(format!("node `{}` has no acceptance targets", node.task_id));
        }
        if node.proof_targets.is_empty() {
            blocker_codes.push("missing_proof_targets".to_string());
            issues.push(format!("node `{}` has no proof targets", node.task_id));
        }
        if node.execution_semantics.execution_mode.as_deref().is_none() {
            blocker_codes.push("missing_execution_semantics".to_string());
            issues.push(format!("node `{}` has no execution_mode", node.task_id));
        }
        if node.execution_semantics.order_bucket.as_deref().is_none() {
            blocker_codes.push("missing_order_bucket".to_string());
            issues.push(format!("node `{}` has no order_bucket", node.task_id));
        }
        if node
            .execution_semantics
            .conflict_domain
            .as_deref()
            .is_none()
        {
            blocker_codes.push("missing_conflict_domain".to_string());
            issues.push(format!("node `{}` has no conflict_domain", node.task_id));
        }
        if node.lane_hint.trim().is_empty() {
            blocker_codes.push("missing_lane_hint".to_string());
            issues.push(format!("node `{}` has no lane hint", node.task_id));
        }
        if let Some(parent_id) = node.parent_id.as_deref() {
            if !allow_unresolved_external_parents
                && !node_ids.contains(parent_id)
                && !existing_ids.contains(parent_id)
            {
                blocker_codes.push("missing_parent".to_string());
                issues.push(format!(
                    "node `{}` parent `{parent_id}` is missing",
                    node.task_id
                ));
            }
        }
        if let Some(existing_task) = existing_by_id.get(node.task_id.as_str()) {
            for reason in existing_task_conflicts(existing_task, node) {
                blocker_codes.push("existing_task_conflict".to_string());
                issues.push(format!(
                    "existing task `{}` conflicts with PlanGraph draft: {reason}",
                    node.task_id
                ));
            }
        }
    }
    for edge in &draft.edges {
        if edge.edge_type.trim().is_empty() {
            blocker_codes.push("missing_edge_type".to_string());
            issues.push(format!(
                "edge `{}` -> `{}` has no edge_type",
                edge.task_id, edge.depends_on_id
            ));
        }
        if !node_ids.contains(edge.task_id.as_str())
            && !existing_ids.contains(edge.task_id.as_str())
        {
            blocker_codes.push("missing_edge_source".to_string());
            issues.push(format!("edge source `{}` is missing", edge.task_id));
        }
        if !node_ids.contains(edge.depends_on_id.as_str())
            && !existing_ids.contains(edge.depends_on_id.as_str())
        {
            blocker_codes.push("missing_dependency".to_string());
            issues.push(format!(
                "edge dependency target `{}` is missing",
                edge.depends_on_id
            ));
        }
        if edge.task_id == edge.depends_on_id {
            blocker_codes.push("self_dependency".to_string());
            issues.push(format!("edge `{}` depends on itself", edge.task_id));
        }
    }
    for cycle in draft_cycles(draft) {
        blocker_codes.push("cyclic_dependency".to_string());
        issues.push(format!("draft dependency cycle detected: {cycle}"));
    }
    let mut path_owners = BTreeMap::<&str, &str>::new();
    for node in &draft.nodes {
        for path in &node.owned_paths {
            if let Some(previous) = path_owners.insert(path.as_str(), node.task_id.as_str()) {
                if previous != node.task_id {
                    blocker_codes.push("conflicting_owned_paths".to_string());
                    issues.push(format!(
                        "owned path `{path}` is claimed by `{previous}` and `{}`",
                        node.task_id
                    ));
                }
            }
        }
    }
    blocker_codes.sort();
    blocker_codes.dedup();
    TaskPlanValidation {
        status: if issues.is_empty() {
            "valid".to_string()
        } else {
            "blocked".to_string()
        },
        blocker_codes,
        issues,
    }
}

fn existing_task_conflicts(existing: &TaskRecord, node: &TaskPlanNodeDraft) -> Vec<String> {
    let mut conflicts = Vec::new();
    if existing.title != node.title {
        conflicts.push(format!("title `{}` != `{}`", existing.title, node.title));
    }
    if existing.issue_type != node.issue_type {
        conflicts.push(format!(
            "issue_type `{}` != `{}`",
            existing.issue_type, node.issue_type
        ));
    }
    if existing.status != node.status {
        conflicts.push(format!("status `{}` != `{}`", existing.status, node.status));
    }
    if existing.priority != node.priority {
        conflicts.push(format!(
            "priority `{}` != `{}`",
            existing.priority, node.priority
        ));
    }
    if normalized_labels(&existing.labels) != normalized_labels(&node.labels) {
        conflicts.push("labels differ".to_string());
    }
    if existing.execution_semantics != node.execution_semantics {
        conflicts.push("execution semantics differ".to_string());
    }
    if existing_parent_id(existing) != node.parent_id.as_deref() {
        conflicts.push(format!(
            "parent `{}` != `{}`",
            existing_parent_id(existing).unwrap_or("<none>"),
            node.parent_id.as_deref().unwrap_or("<none>")
        ));
    }
    let expected_metadata = planner_metadata_from_node(node);
    if existing.description != node.description {
        conflicts.push("description differs".to_string());
    }
    if existing.planner_metadata != expected_metadata {
        conflicts.push("planner metadata differs".to_string());
    }
    conflicts
}

fn existing_parent_id(task: &TaskRecord) -> Option<&str> {
    task.dependencies
        .iter()
        .find(|dependency| dependency.edge_type == "parent-child")
        .map(|dependency| dependency.depends_on_id.as_str())
}

fn normalized_labels(labels: &[String]) -> Vec<String> {
    let mut normalized = labels
        .iter()
        .map(|label| label.trim().to_string())
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn draft_cycles(draft: &TaskPlanGraphDraft) -> Vec<String> {
    let node_ids = draft
        .nodes
        .iter()
        .map(|node| node.task_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut adjacency = BTreeMap::<&str, Vec<&str>>::new();
    for edge in &draft.edges {
        if node_ids.contains(edge.task_id.as_str())
            && node_ids.contains(edge.depends_on_id.as_str())
        {
            adjacency
                .entry(edge.task_id.as_str())
                .or_default()
                .push(edge.depends_on_id.as_str());
        }
    }

    let mut cycles = Vec::new();
    for node_id in &node_ids {
        let mut path = Vec::new();
        let mut active = BTreeSet::new();
        collect_draft_cycles(node_id, &adjacency, &mut path, &mut active, &mut cycles);
    }
    cycles.sort();
    cycles.dedup();
    cycles
}

fn collect_draft_cycles<'a>(
    node_id: &'a str,
    adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
    path: &mut Vec<&'a str>,
    active: &mut BTreeSet<&'a str>,
    cycles: &mut Vec<String>,
) {
    if active.contains(node_id) {
        if let Some(index) = path.iter().position(|value| *value == node_id) {
            let mut cycle = path[index..].to_vec();
            cycle.push(node_id);
            cycles.push(cycle.join(" -> "));
        }
        return;
    }
    active.insert(node_id);
    path.push(node_id);
    if let Some(next_ids) = adjacency.get(node_id) {
        for next_id in next_ids {
            collect_draft_cycles(next_id, adjacency, path, active, cycles);
        }
    }
    path.pop();
    active.remove(node_id);
}

fn simulate_materialized_graph(
    draft: &TaskPlanGraphDraft,
    existing: &[TaskRecord],
) -> Vec<TaskGraphIssue> {
    let mut rows = existing.to_vec();
    let existing_ids = existing
        .iter()
        .map(|task| task.id.as_str())
        .collect::<BTreeSet<_>>();
    for node in &draft.nodes {
        if existing_ids.contains(node.task_id.as_str()) {
            continue;
        }
        let mut dependencies = Vec::new();
        if let Some(parent_id) = node.parent_id.as_deref() {
            dependencies.push(TaskDependencyRecord {
                issue_id: node.task_id.clone(),
                depends_on_id: parent_id.to_string(),
                edge_type: "parent-child".to_string(),
                created_at: "0".to_string(),
                created_by: "vida taskflow plan materialize dry-run".to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            });
        }
        rows.push(TaskRecord {
            id: node.task_id.clone(),
            display_id: None,
            title: node.title.clone(),
            description: node.description.clone(),
            status: node.status.clone(),
            priority: node.priority,
            issue_type: node.issue_type.clone(),
            created_at: "0".to_string(),
            created_by: "vida taskflow plan materialize dry-run".to_string(),
            updated_at: "0".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: "vida-stack".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: node.labels.clone(),
            execution_semantics: node.execution_semantics.clone(),
            planner_metadata: planner_metadata_from_node(node),
            dependencies,
        });
    }
    for edge in &draft.edges {
        if let Some(task) = rows.iter_mut().find(|task| task.id == edge.task_id) {
            if task.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == edge.depends_on_id
                    && dependency.edge_type == edge.edge_type
            }) {
                continue;
            }
            task.dependencies.push(TaskDependencyRecord {
                issue_id: edge.task_id.clone(),
                depends_on_id: edge.depends_on_id.clone(),
                edge_type: edge.edge_type.clone(),
                created_at: "0".to_string(),
                created_by: "vida taskflow plan materialize dry-run".to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            });
        }
    }
    StateStore::validate_task_graph_rows(&rows)
}

#[cfg(test)]
fn task_description_with_plan_metadata(node: &TaskPlanNodeDraft) -> String {
    format!(
        "{}\n\nPlanGraph metadata:\nowned_paths:\n{}\nacceptance_targets:\n{}\nproof_targets:\n{}\nrisk: {}\nestimate: {}\nlane_hint: {}",
        node.description,
        bullet_list(&node.owned_paths),
        bullet_list(&node.acceptance_targets),
        bullet_list(&node.proof_targets),
        node.risk,
        node.estimate,
        node.lane_hint
    )
}

fn planner_metadata_from_node(node: &TaskPlanNodeDraft) -> TaskPlannerMetadata {
    TaskPlannerMetadata {
        owned_paths: node.owned_paths.clone(),
        acceptance_targets: node.acceptance_targets.clone(),
        proof_targets: node.proof_targets.clone(),
        risk: Some(node.risk.clone()).filter(|value| !value.trim().is_empty()),
        estimate: Some(node.estimate.clone()).filter(|value| !value.trim().is_empty()),
        lane_hint: Some(node.lane_hint.clone()).filter(|value| !value.trim().is_empty()),
    }
}

#[cfg(test)]
fn bullet_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("- {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_draft(path: &Path) -> Result<TaskPlanGraphDraft, String> {
    let raw = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&raw).map_err(|error| error.to_string())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let body = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    std::fs::write(path, format!("{body}\n")).map_err(|error| error.to_string())
}

fn print_json_or_plain<T: Serialize>(json: bool, value: &T, plain: &str) -> ExitCode {
    if json {
        match serde_json::to_string_pretty(value) {
            Ok(body) => println!("{body}"),
            Err(error) => {
                eprintln!("failed to render json: {error}");
                return ExitCode::from(1);
            }
        }
    } else {
        println!("{plain}");
    }
    ExitCode::SUCCESS
}

fn normalize_source(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn first_meaningful_line(value: &str) -> Option<&str> {
    value
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.trim_start_matches('#').trim())
}

fn summarize_source(value: &str) -> String {
    let mut summary = value
        .split_whitespace()
        .take(28)
        .collect::<Vec<_>>()
        .join(" ");
    if summary.len() > 240 {
        summary.truncate(240);
    }
    summary
}

fn normalize_task_id_segment(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn stable_hash_hex(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_generate_is_deterministic_for_same_input() {
        let options = PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner\n\nwith deterministic output".to_string()),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: Some("parent-task".to_string()),
            output: None,
            json: true,
        };
        let first = generate_plan_graph_draft(&options).expect("draft should generate");
        let second = generate_plan_graph_draft(&options).expect("draft should generate");
        assert_eq!(first, second);
        assert_eq!(first.validation.status, "valid");
        assert!(first.validation.blocker_codes.is_empty());
    }

    #[test]
    fn draft_validation_rejects_missing_dependency() {
        let mut draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");
        let dependent_task_id = draft
            .nodes
            .last()
            .expect("draft should contain at least one node")
            .task_id
            .clone();
        draft.edges.push(TaskPlanEdgeDraft {
            task_id: dependent_task_id,
            depends_on_id: "missing".to_string(),
            edge_type: "blocks".to_string(),
            reason: "test".to_string(),
        });
        let validation = validate_draft(&draft, &[]);
        assert_eq!(validation.status, "blocked");
        assert!(
            validation
                .blocker_codes
                .contains(&"missing_dependency".to_string())
        );
    }

    #[test]
    fn plan_generate_extracts_bounded_items_from_report_split() {
        let draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some(
                "Final gap report\n\nRecommended split for backlog:\n- real PlanGraph generation (`crates/vida/src/taskflow_plan_graph.rs:252-385`, `crates/vida/src/taskflow_proxy.rs:1215-1400`)\n- structured planner metadata (`crates/vida/src/state_store_task_models.rs:151-205`)\n- scheduler dispatch with `max_parallel_agents` (`crates/vida/src/taskflow_consume_bundle.rs:709-760`)\n".to_string(),
            ),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");

        assert_eq!(draft.validation.status, "valid");
        assert_eq!(draft.nodes.len(), 3);
        assert_eq!(draft.edges.len(), 2);
        assert!(draft.nodes[0].task_id.starts_with("feature-planner-"));
        assert!(draft.nodes.iter().any(|node| {
            node.owned_paths
                .contains(&"crates/vida/src/taskflow_plan_graph.rs".to_string())
                && node
                    .owned_paths
                    .contains(&"crates/vida/src/taskflow_proxy.rs".to_string())
        }));
        assert_eq!(
            draft.nodes[1].owned_paths,
            vec!["crates/vida/src/state_store_task_models.rs".to_string()]
        );
        assert!(
            draft.nodes[2]
                .owned_paths
                .contains(&"crates/vida/src/taskflow_consume_bundle.rs".to_string())
        );
        assert_eq!(draft.edges[0].task_id, draft.nodes[1].task_id);
        assert_eq!(draft.edges[0].depends_on_id, draft.nodes[0].task_id);
        assert_eq!(draft.edges[1].task_id, draft.nodes[2].task_id);
        assert_eq!(draft.edges[1].depends_on_id, draft.nodes[1].task_id);
        assert!(draft.nodes.iter().all(|node| {
            node.operator_truth
                .iter()
                .any(|marker| marker == "owned_paths_source=explicit_item_paths")
        }));
        assert!(draft.nodes.iter().all(|node| {
            !node.operator_truth.iter().any(|marker| {
                marker
                    == "low_confidence_owned_paths_template_fallback_requires_operator_confirmation"
            })
        }));
        assert!(draft.nodes.iter().all(|node| {
            node.operator_truth.iter().any(|marker| {
                marker
                    == "low_confidence_proof_targets_template_guess_requires_operator_confirmation"
            })
        }));
    }

    #[test]
    fn plan_generate_falls_back_to_semantic_decomposition_without_split_block() {
        let draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some(
                "Implement adaptive scheduler dispatch with state metadata and proof coverage"
                    .to_string(),
            ),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");

        assert_eq!(draft.validation.status, "valid");
        assert!(draft.nodes.len() >= 4);
        assert!(
            draft
                .nodes
                .iter()
                .any(|node| node.title.contains("Wire task state and schema surfaces"))
        );
        assert!(draft.nodes.iter().any(|node| {
            node.title
                .contains("Wire runtime and orchestration surfaces")
        }));
        assert!(
            draft
                .nodes
                .iter()
                .any(|node| node.title.starts_with("Prove "))
        );
        assert_eq!(draft.edges.len(), draft.nodes.len() - 1);
    }

    #[test]
    fn plan_generate_marks_template_fallback_paths_and_proofs_low_confidence() {
        let draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");

        assert_eq!(draft.validation.status, "valid");
        assert!(
            draft
                .nodes
                .iter()
                .all(|node| node.evidence_confidence == "low")
        );
        assert!(draft.nodes.iter().all(|node| {
            node.operator_truth
                .iter()
                .any(|marker| marker == "owned_paths_source=template_fallback")
        }));
        assert!(draft.nodes.iter().all(|node| {
            node.operator_truth.iter().any(|marker| {
                marker
                    == "low_confidence_owned_paths_template_fallback_requires_operator_confirmation"
            })
        }));
        assert!(draft.nodes.iter().all(|node| {
            node.operator_truth.iter().any(|marker| {
                marker
                    == "low_confidence_proof_targets_template_guess_requires_operator_confirmation"
            })
        }));
    }

    #[test]
    fn plan_generate_input_contract_reports_provided_spec_backlog_and_context_refs() {
        let draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some(
                "Implement audit-p1-planner-context-input-contract-remediation using \
                 docs/product/spec/current-spec-map.md and \
                 crates/vida/src/taskflow_plan_graph.rs"
                    .to_string(),
            ),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");

        assert_eq!(draft.input_contract.status, "complete");
        assert!(draft.input_contract.missing_context.is_empty());
        assert!(draft.input_contract.sources.iter().any(|source| {
            source.source_type == "primary_source"
                && source.reference == "text"
                && source.status == "provided"
        }));
        assert!(draft.input_contract.sources.iter().any(|source| {
            source.source_type == "spec_reference"
                && source.reference == "docs/product/spec/current-spec-map.md"
                && source.status == "provided"
        }));
        assert!(draft.input_contract.sources.iter().any(|source| {
            source.source_type == "context_reference"
                && source.reference == "crates/vida/src/taskflow_plan_graph.rs"
                && source.status == "provided"
        }));
        assert!(draft.input_contract.sources.iter().any(|source| {
            source.source_type == "backlog_reference"
                && source.reference == "audit-p1-planner-context-input-contract-remediation"
                && source.status == "provided"
        }));
        assert!(
            draft
                .input_contract
                .operator_truth
                .contains(&"planner_input_contract_foundation_only".to_string())
        );
    }

    #[test]
    fn plan_generate_input_contract_marks_missing_context_refs() {
        let draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");

        assert_eq!(draft.input_contract.status, "partial");
        assert!(draft.input_contract.sources.iter().any(|source| {
            source.source_type == "primary_source"
                && source.reference == "text"
                && source.status == "provided"
        }));
        for missing in [
            "spec_reference_missing",
            "backlog_reference_missing",
            "context_reference_missing",
        ] {
            assert!(
                draft
                    .input_contract
                    .missing_context
                    .contains(&missing.to_string())
            );
            assert!(
                draft
                    .input_contract
                    .operator_truth
                    .contains(&format!("{missing}_requires_operator_confirmation"))
            );
        }
    }

    #[test]
    fn parse_generate_options_accepts_repeatable_input_contract_refs() {
        let args = vec![
            "vida".to_string(),
            "generate".to_string(),
            "--source-text".to_string(),
            "Implement planner".to_string(),
            "--task-prefix".to_string(),
            "feature-planner".to_string(),
            "--spec-ref".to_string(),
            "docs/product/spec/current-spec-map.md".to_string(),
            "--spec-ref".to_string(),
            "docs/product/spec/runtime.md".to_string(),
            "--backlog-ref".to_string(),
            "audit-p1-planner-input-contract-cli-flags".to_string(),
            "--context-ref".to_string(),
            "crates/vida/src/taskflow_plan_graph.rs".to_string(),
            "--json".to_string(),
        ];

        let options = parse_generate_options(&args).expect("options should parse");

        assert_eq!(
            options.spec_refs,
            vec![
                "docs/product/spec/current-spec-map.md".to_string(),
                "docs/product/spec/runtime.md".to_string()
            ]
        );
        assert_eq!(
            options.backlog_refs,
            vec!["audit-p1-planner-input-contract-cli-flags".to_string()]
        );
        assert_eq!(
            options.context_refs,
            vec!["crates/vida/src/taskflow_plan_graph.rs".to_string()]
        );
        assert!(options.json);
    }

    #[test]
    fn plan_generate_input_contract_uses_cli_refs_for_missing_source_context() {
        let draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            spec_refs: vec!["docs/product/spec/current-spec-map.md".to_string()],
            backlog_refs: vec!["audit-p1-planner-input-contract-cli-flags".to_string()],
            context_refs: vec!["crates/vida/src/taskflow_plan_graph.rs".to_string()],
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");

        assert_eq!(draft.input_contract.status, "complete");
        assert!(draft.input_contract.missing_context.is_empty());
        assert!(draft.input_contract.sources.iter().any(|source| {
            source.source_type == "spec_reference"
                && source.reference == "docs/product/spec/current-spec-map.md"
                && source.evidence == "cli_spec_ref"
        }));
        assert!(draft.input_contract.sources.iter().any(|source| {
            source.source_type == "backlog_reference"
                && source.reference == "audit-p1-planner-input-contract-cli-flags"
                && source.evidence == "cli_backlog_ref"
        }));
        assert!(draft.input_contract.sources.iter().any(|source| {
            source.source_type == "context_reference"
                && source.reference == "crates/vida/src/taskflow_plan_graph.rs"
                && source.evidence == "cli_context_ref"
        }));
    }

    #[test]
    fn plan_generate_orders_bounded_items_sequentially() {
        let draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");

        assert_eq!(draft.validation.status, "valid");
        assert_eq!(draft.edges.len(), draft.nodes.len() - 1);
        for (edge, pair) in draft.edges.iter().zip(draft.nodes.windows(2)) {
            assert_eq!(edge.task_id, pair[1].task_id);
            assert_eq!(edge.depends_on_id, pair[0].task_id);
        }
        assert_eq!(
            draft
                .nodes
                .last()
                .expect("draft should contain a proof node")
                .lane_hint,
            "verifier"
        );
    }

    #[test]
    fn draft_validation_accepts_parent_declared_later_in_draft() {
        let mut draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");
        draft.nodes[0].parent_id = Some(draft.nodes[1].task_id.clone());

        let validation = validate_draft(&draft, &[]);

        assert_eq!(validation.status, "valid");
    }

    #[test]
    fn draft_validation_rejects_cycle_before_materialization() {
        let mut draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");
        draft.edges.push(TaskPlanEdgeDraft {
            task_id: draft.nodes[0].task_id.clone(),
            depends_on_id: draft.nodes[1].task_id.clone(),
            edge_type: "blocks".to_string(),
            reason: "test cycle".to_string(),
        });

        let validation = validate_draft(&draft, &[]);

        assert_eq!(validation.status, "blocked");
        assert!(
            validation
                .blocker_codes
                .contains(&"cyclic_dependency".to_string())
        );
    }

    #[tokio::test]
    async fn dry_run_receipt_separates_created_and_existing_tasks() {
        let state_dir = test_state_dir("dry-run-receipt");
        let store = StateStore::open(state_dir.clone())
            .await
            .expect("state store should open");
        let draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");
        let existing_node = &draft.nodes[0];
        store
            .create_task(CreateTaskRequest {
                task_id: &existing_node.task_id,
                title: &existing_node.title,
                display_id: None,
                description: &existing_node.description,
                issue_type: &existing_node.issue_type,
                status: &existing_node.status,
                priority: existing_node.priority,
                parent_id: None,
                labels: &existing_node.labels,
                execution_semantics: existing_node.execution_semantics.clone(),
                planner_metadata: planner_metadata_from_node(existing_node),
                created_by: "test",
                source_repo: "vida-stack",
            })
            .await
            .expect("existing task should create");
        drop(store);

        let receipt = materialize_plan_graph_draft(
            &draft,
            &PlanMaterializeOptions {
                draft_path: PathBuf::from("unused.json"),
                state_dir: state_dir.clone(),
                dry_run: true,
                json: true,
            },
        )
        .await
        .expect("dry-run should produce receipt");

        assert_eq!(receipt.status, "dry_run");
        assert_eq!(
            receipt.skipped_existing_task_ids,
            vec![draft.nodes[0].task_id.clone()]
        );
        assert_eq!(
            receipt.created_task_ids,
            draft
                .nodes
                .iter()
                .skip(1)
                .map(|node| node.task_id.clone())
                .collect::<Vec<_>>()
        );
        assert!(receipt.graph_validation.is_empty());
        let _ = std::fs::remove_dir_all(state_dir);
    }

    #[tokio::test]
    async fn materialize_blocks_existing_task_collision_with_different_parent() {
        let state_dir = test_state_dir("existing-collision");
        let store = StateStore::open(state_dir.clone())
            .await
            .expect("state store should open");
        let empty_labels = Vec::<String>::new();
        for parent_id in ["parent-a", "parent-b"] {
            store
                .create_task(CreateTaskRequest {
                    task_id: parent_id,
                    title: parent_id,
                    display_id: None,
                    description: parent_id,
                    issue_type: "epic",
                    status: "open",
                    priority: 1,
                    parent_id: None,
                    labels: &empty_labels,
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "vida-stack",
                })
                .await
                .expect("parent should create");
        }
        let draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: Some("parent-b".to_string()),
            output: None,
            json: true,
        })
        .expect("draft should generate");
        let existing_node = &draft.nodes[0];
        store
            .create_task(CreateTaskRequest {
                task_id: &existing_node.task_id,
                title: &existing_node.title,
                display_id: None,
                description: &existing_node.description,
                issue_type: &existing_node.issue_type,
                status: &existing_node.status,
                priority: existing_node.priority,
                parent_id: Some("parent-a"),
                labels: &existing_node.labels,
                execution_semantics: existing_node.execution_semantics.clone(),
                planner_metadata: planner_metadata_from_node(existing_node),
                created_by: "test",
                source_repo: "vida-stack",
            })
            .await
            .expect("existing task should create");
        drop(store);

        let receipt = materialize_plan_graph_draft(
            &draft,
            &PlanMaterializeOptions {
                draft_path: PathBuf::from("unused.json"),
                state_dir: state_dir.clone(),
                dry_run: false,
                json: true,
            },
        )
        .await
        .expect("collision should produce blocked receipt");

        assert_eq!(receipt.status, "blocked");
        assert!(receipt.created_task_ids.is_empty());
        assert!(
            receipt
                .validation
                .blocker_codes
                .contains(&"existing_task_conflict".to_string())
        );
        let _ = std::fs::remove_dir_all(state_dir);
    }

    #[tokio::test]
    async fn dry_run_receipt_blocks_legacy_description_embedded_metadata_without_structured_metadata()
     {
        let state_dir = test_state_dir("legacy-description-metadata-blocked");
        let store = StateStore::open(state_dir.clone())
            .await
            .expect("state store should open");
        let draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");
        let existing_node = &draft.nodes[0];
        let legacy_description = task_description_with_plan_metadata(existing_node);

        store
            .create_task(CreateTaskRequest {
                task_id: &existing_node.task_id,
                title: &existing_node.title,
                display_id: None,
                description: &legacy_description,
                issue_type: &existing_node.issue_type,
                status: &existing_node.status,
                priority: existing_node.priority,
                parent_id: None,
                labels: &existing_node.labels,
                execution_semantics: existing_node.execution_semantics.clone(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "vida-stack",
            })
            .await
            .expect("legacy description-only task should create");
        drop(store);

        let receipt = materialize_plan_graph_draft(
            &draft,
            &PlanMaterializeOptions {
                draft_path: PathBuf::from("unused.json"),
                state_dir: state_dir.clone(),
                dry_run: true,
                json: true,
            },
        )
        .await
        .expect("legacy description-only dry-run should return blocked receipt");

        assert_eq!(receipt.status, "blocked");
        assert!(receipt.created_task_ids.is_empty());
        assert!(receipt.skipped_existing_task_ids.is_empty());
        assert!(
            receipt
                .validation
                .blocker_codes
                .contains(&"existing_task_conflict".to_string())
        );
        assert!(
            receipt
                .validation
                .issues
                .iter()
                .any(|issue| issue.contains("conflicts with PlanGraph draft: description differs"))
        );
        assert!(receipt.validation.issues.iter().any(|issue| {
            issue.contains("conflicts with PlanGraph draft: planner metadata differs")
        }));
        assert!(receipt.graph_validation.is_empty());
        let _ = std::fs::remove_dir_all(state_dir);
    }

    #[tokio::test]
    async fn materialize_blocks_invalid_graph_before_writing() {
        let state_dir = test_state_dir("invalid-graph");
        let mut draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            spec_refs: Vec::new(),
            backlog_refs: Vec::new(),
            context_refs: Vec::new(),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");
        draft.edges.push(TaskPlanEdgeDraft {
            task_id: draft.nodes[0].task_id.clone(),
            depends_on_id: draft.nodes[1].task_id.clone(),
            edge_type: "blocks".to_string(),
            reason: "test cycle".to_string(),
        });

        let receipt = materialize_plan_graph_draft(
            &draft,
            &PlanMaterializeOptions {
                draft_path: PathBuf::from("unused.json"),
                state_dir: state_dir.clone(),
                dry_run: false,
                json: true,
            },
        )
        .await
        .expect("invalid draft should produce blocked receipt");

        assert_eq!(receipt.status, "blocked");
        assert!(receipt.created_task_ids.is_empty());
        assert!(
            receipt
                .validation
                .blocker_codes
                .contains(&"cyclic_dependency".to_string())
        );
        let store = StateStore::open_existing(state_dir.clone())
            .await
            .expect("state store should open");
        assert!(
            store
                .list_tasks(None, true)
                .await
                .expect("tasks should list")
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(state_dir);
    }

    fn test_state_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "vida-taskflow-plan-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        path
    }
}
