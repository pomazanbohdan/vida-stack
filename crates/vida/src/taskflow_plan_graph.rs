use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::{Deserialize, Serialize};

use crate::state_store::{
    CreateTaskRequest, StateStore, StateStoreError, TaskDependencyRecord, TaskExecutionSemantics,
    TaskGraphIssue, TaskRecord,
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
    pub nodes: Vec<TaskPlanNodeDraft>,
    pub edges: Vec<TaskPlanEdgeDraft>,
    pub validation: TaskPlanValidation,
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
        "TaskFlow PlanGraph surfaces\n\n  vida taskflow plan generate --source-file <path> --task-prefix <prefix> --json\n  vida taskflow plan generate --source-text <text> --task-prefix <prefix> --json\n  vida taskflow plan materialize <draft.json> --dry-run --json\n  vida taskflow plan materialize <draft.json> --json"
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
            "--help" | "-h" => return Err("usage: vida taskflow plan generate --source-file <path>|--source-text <text> --task-prefix <prefix> [--parent-id <id>] [--output <path>] [--json]".to_string()),
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
    let nodes = build_nodes(&task_prefix, options.parent_id.clone(), &normalized_source);
    let mut edges = Vec::new();
    for pair in nodes.windows(2) {
        edges.push(TaskPlanEdgeDraft {
            task_id: pair[1].task_id.clone(),
            depends_on_id: pair[0].task_id.clone(),
            edge_type: "blocks".to_string(),
            reason: "deterministic sequential proof dependency".to_string(),
        });
    }
    let mut draft = TaskPlanGraphDraft {
        draft_id: format!("plan-{task_prefix}-{source_hash}"),
        source_kind,
        source_ref,
        source_hash,
        task_prefix,
        parent_id: options.parent_id.clone(),
        generated_by: "vida taskflow plan generate".to_string(),
        deterministic: true,
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

fn build_nodes(
    task_prefix: &str,
    parent_id: Option<String>,
    normalized_source: &str,
) -> Vec<TaskPlanNodeDraft> {
    let title = first_meaningful_line(normalized_source)
        .unwrap_or("Implement bounded PlanGraph work")
        .to_string();
    let summary = summarize_source(normalized_source);
    vec![
        TaskPlanNodeDraft {
            task_id: format!("{task_prefix}-plan-surfaces"),
            title: format!("Implement {title} plan surfaces"),
            description: format!(
                "Implement deterministic PlanGraph generation/materialization for: {summary}"
            ),
            issue_type: "delivery_task".to_string(),
            status: "open".to_string(),
            priority: 1,
            parent_id: parent_id.clone(),
            labels: vec!["taskflow".to_string(), "plan-graph".to_string()],
            execution_semantics: TaskExecutionSemantics {
                execution_mode: Some("sequential".to_string()),
                order_bucket: Some("plan-graph-phase-1".to_string()),
                parallel_group: None,
                conflict_domain: Some("taskflow-plan-graph".to_string()),
            },
            owned_paths: vec![
                "crates/vida/src/taskflow_plan_graph.rs".to_string(),
                "crates/vida/src/taskflow_proxy.rs".to_string(),
            ],
            acceptance_targets: vec![
                "vida taskflow plan generate emits deterministic PlanGraph JSON".to_string(),
                "vida taskflow plan materialize writes through canonical task store APIs"
                    .to_string(),
            ],
            proof_targets: vec![
                "cargo test -p vida taskflow_plan_graph".to_string(),
                "vida taskflow plan generate --json".to_string(),
            ],
            risk: "medium".to_string(),
            estimate: "bounded implementation packet".to_string(),
            lane_hint: "implementer".to_string(),
        },
        TaskPlanNodeDraft {
            task_id: format!("{task_prefix}-plan-proof"),
            title: format!("Prove {title} plan materialization"),
            description: format!(
                "Validate deterministic PlanGraph receipts and graph safety for: {summary}"
            ),
            issue_type: "delivery_task".to_string(),
            status: "open".to_string(),
            priority: 2,
            parent_id,
            labels: vec!["taskflow".to_string(), "proof".to_string()],
            execution_semantics: TaskExecutionSemantics {
                execution_mode: Some("sequential".to_string()),
                order_bucket: Some("plan-graph-phase-1".to_string()),
                parallel_group: None,
                conflict_domain: Some("taskflow-plan-graph".to_string()),
            },
            owned_paths: vec!["crates/vida/tests".to_string()],
            acceptance_targets: vec![
                "materialization rejects invalid draft dependencies".to_string(),
                "materialization receipt reports created/skipped tasks and dependency edges"
                    .to_string(),
            ],
            proof_targets: vec![
                "cargo test -p vida taskflow_plan_graph".to_string(),
                "vida task validate-graph --json".to_string(),
            ],
            risk: "medium".to_string(),
            estimate: "bounded verification packet".to_string(),
            lane_hint: "verifier".to_string(),
        },
    ]
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
        let description = task_description_with_plan_metadata(node);
        let source_repo = std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "vida-stack".to_string());
        store
            .create_task(CreateTaskRequest {
                task_id: &node.task_id,
                title: &node.title,
                display_id: None,
                description: &description,
                issue_type: &node.issue_type,
                status: &node.status,
                priority: node.priority,
                parent_id: node.parent_id.as_deref(),
                labels: &node.labels,
                execution_semantics: node.execution_semantics.clone(),
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
    let expected_description = task_description_with_plan_metadata(node);
    if existing.description != expected_description {
        conflicts.push("planner metadata description differs".to_string());
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
            description: task_description_with_plan_metadata(node),
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
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");
        draft.edges.push(TaskPlanEdgeDraft {
            task_id: "feature-planner-plan-proof".to_string(),
            depends_on_id: "missing".to_string(),
            edge_type: "blocks".to_string(),
            reason: "test".to_string(),
        });
        let validation = validate_draft(&draft, &[]);
        assert_eq!(validation.status, "blocked");
        assert!(validation
            .blocker_codes
            .contains(&"missing_dependency".to_string()));
    }

    #[test]
    fn plan_generate_orders_implementation_before_proof() {
        let draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");

        assert_eq!(draft.validation.status, "valid");
        assert_eq!(draft.nodes[0].task_id, "feature-planner-plan-surfaces");
        assert_eq!(draft.nodes[1].task_id, "feature-planner-plan-proof");
        assert_eq!(draft.edges[0].task_id, "feature-planner-plan-proof");
        assert_eq!(
            draft.edges[0].depends_on_id,
            "feature-planner-plan-surfaces"
        );
    }

    #[test]
    fn draft_validation_accepts_parent_declared_later_in_draft() {
        let mut draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
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
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");
        draft.edges.push(TaskPlanEdgeDraft {
            task_id: "feature-planner-plan-surfaces".to_string(),
            depends_on_id: "feature-planner-plan-proof".to_string(),
            edge_type: "blocks".to_string(),
            reason: "test cycle".to_string(),
        });

        let validation = validate_draft(&draft, &[]);

        assert_eq!(validation.status, "blocked");
        assert!(validation
            .blocker_codes
            .contains(&"cyclic_dependency".to_string()));
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
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");
        let existing_node = &draft.nodes[0];
        let existing_description = task_description_with_plan_metadata(existing_node);
        store
            .create_task(CreateTaskRequest {
                task_id: &existing_node.task_id,
                title: &existing_node.title,
                display_id: None,
                description: &existing_description,
                issue_type: &existing_node.issue_type,
                status: &existing_node.status,
                priority: existing_node.priority,
                parent_id: None,
                labels: &existing_node.labels,
                execution_semantics: existing_node.execution_semantics.clone(),
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
            vec!["feature-planner-plan-surfaces".to_string()]
        );
        assert_eq!(
            receipt.created_task_ids,
            vec!["feature-planner-plan-proof".to_string()]
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
                    created_by: "test",
                    source_repo: "vida-stack",
                })
                .await
                .expect("parent should create");
        }
        let draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: Some("parent-b".to_string()),
            output: None,
            json: true,
        })
        .expect("draft should generate");
        let existing_node = &draft.nodes[0];
        let existing_description = task_description_with_plan_metadata(existing_node);
        store
            .create_task(CreateTaskRequest {
                task_id: &existing_node.task_id,
                title: &existing_node.title,
                display_id: None,
                description: &existing_description,
                issue_type: &existing_node.issue_type,
                status: &existing_node.status,
                priority: existing_node.priority,
                parent_id: Some("parent-a"),
                labels: &existing_node.labels,
                execution_semantics: existing_node.execution_semantics.clone(),
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
        assert!(receipt
            .validation
            .blocker_codes
            .contains(&"existing_task_conflict".to_string()));
        let _ = std::fs::remove_dir_all(state_dir);
    }

    #[tokio::test]
    async fn materialize_blocks_invalid_graph_before_writing() {
        let state_dir = test_state_dir("invalid-graph");
        let mut draft = generate_plan_graph_draft(&PlanGenerateOptions {
            source_file: None,
            source_text: Some("Implement planner".to_string()),
            task_prefix: Some("feature-planner".to_string()),
            parent_id: None,
            output: None,
            json: true,
        })
        .expect("draft should generate");
        draft.edges.push(TaskPlanEdgeDraft {
            task_id: "feature-planner-plan-surfaces".to_string(),
            depends_on_id: "feature-planner-plan-proof".to_string(),
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
        assert!(receipt
            .validation
            .blocker_codes
            .contains(&"cyclic_dependency".to_string()));
        let store = StateStore::open_existing(state_dir.clone())
            .await
            .expect("state store should open");
        assert!(store
            .list_tasks(None, true)
            .await
            .expect("tasks should list")
            .is_empty());
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
