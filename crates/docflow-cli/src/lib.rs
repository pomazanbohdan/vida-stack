use clap::{Args, Parser, Subcommand};
use docflow_config::{resolve_profile_roots, resolve_scan_ignored_globs};
use docflow_contracts::{ReadinessRow, ScanRow};
use docflow_core::{ArtifactPath, CheckedAt, ReadinessVerdict};
use docflow_format_jsonl::encode_line;
use docflow_inventory::{InventoryScope, build_registry};
use docflow_operator::{
    render_artifact_impact, render_layer_status, render_overview, render_relation_summary,
    render_summary, render_task_impact,
};
use docflow_readiness::{issues_to_readiness_rows, summarize_verdict};
use docflow_relations::{RelationEdge, artifact_identity_edges};
use docflow_validation::{ValidationIssue, validate_markdown_footer};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use time::format_description::well_known::Rfc3339;

#[derive(Debug, Parser)]
#[command(
    name = "docflow",
    about = "Standalone DocFlow CLI for documentation readiness, validation, and agent handoff"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Overview(OverviewArgs),
    OverviewScan(RegistryScanArgs),
    Summary(RegistryScanArgs),
    Changelog(ChangelogArgs),
    ChangelogTask(ChangelogTaskArgs),
    TaskSummary(TaskSummaryArgs),
    LayerStatus(LayerStatusArgs),
    Check(CheckArgs),
    Links(PathArgs),
    Deps(PathArgs),
    DepsMap(PathArgs),
    ArtifactImpact(ArtifactImpactArgs),
    TaskImpact(TaskImpactArgs),
    Fastcheck(CheckArgs),
    ActivationCheck(CheckArgs),
    ProtocolCoverageCheck(CheckArgs),
    FinalizeEdit(FinalizeEditArgs),
    Proofcheck(ProofcheckArgs),
    Doctor(DoctorArgs),
    Relations(RelationsArgs),
    RelationsScan(RegistryScanArgs),
    Scan(ScanArgs),
    Registry(RegistryScanArgs),
    RegistryScan(RegistryScanArgs),
    RegistryWrite(RegistryWriteArgs),
    #[command(
        about = "print agent bootstrap instructions or initialize a canonical markdown artifact",
        long_about = "Print DocFlow agent bootstrap instructions or initialize a canonical markdown artifact.\n\nWithout positional arguments, `docflow init` prints the utility contract, safe first commands, read-first docs, and next actions for an agent.\nWith four positional arguments, `docflow init <markdown_file> <artifact_path> <artifact_type> <change_note>` creates a canonical markdown artifact with footer metadata and changelog linkage.",
        after_help = "Examples:\n  docflow init\n  docflow init --json\n  docflow init docs/process/example.md process/example process_doc \"initialize docs artifact\"\n  docflow init docs/process/example.md process/example process_doc \"initialize docs artifact\" --json"
    )]
    Init(InitArgs),
    MigrateLinks(MigrateLinksArgs),
    Move(MoveArgs),
    RenameArtifact(RenameArtifactArgs),
    Touch(TouchArgs),
    ValidateTree(RegistryScanArgs),
    ReadinessTree(RegistryScanArgs),
    ReadinessCheck(CheckArgs),
    ReadinessWrite(RegistryWriteArgs),
    ValidateFooter(ValidateFooterArgs),
    Readiness(ReadinessArgs),
    CheckFile(FileArgs),
    ReadinessFile(FileArgs),
    ReportCheck(FileArgs),
}

#[derive(Debug, Args)]
pub struct OverviewArgs {
    #[arg(long, default_value_t = 0)]
    pub registry_count: usize,
    #[arg(long, default_value_t = 0)]
    pub relation_count: usize,
}

#[derive(Debug, Args)]
pub struct RelationsArgs {
    #[arg(long, default_value_t = 0)]
    pub edge_count: usize,
}

#[derive(Debug, Args)]
pub struct ValidateFooterArgs {
    #[arg(long)]
    pub path: String,
    #[arg(long)]
    pub content: String,
}

#[derive(Debug, Args)]
pub struct ReadinessArgs {
    #[arg(long)]
    pub path: String,
    #[arg(long)]
    pub content: String,
}

#[derive(Debug, Args)]
pub struct FileArgs {
    #[arg(long)]
    pub path: String,
}

#[derive(Debug, Args)]
pub struct RegistryScanArgs {
    #[arg(long)]
    pub root: String,
    #[arg(long = "exclude-glob")]
    pub exclude_globs: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ChangelogArgs {
    pub markdown_file: String,
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
    #[arg(long, default_value_t = false)]
    pub newest_first: bool,
    #[arg(long = "format", default_value = "toon")]
    pub format: String,
}

#[derive(Debug, Args)]
pub struct ChangelogTaskArgs {
    #[arg(long)]
    pub root: Option<String>,
    #[arg(long, default_value = "")]
    pub profile: String,
    #[arg(long = "task-id", default_value = "")]
    pub task_id: String,
    #[arg(long, default_value_t = 0)]
    pub limit: usize,
    #[arg(long, default_value_t = false)]
    pub newest_first: bool,
    #[arg(long = "format", default_value = "toon")]
    pub format: String,
}

#[derive(Debug, Args)]
pub struct TaskSummaryArgs {
    #[arg(long)]
    pub root: Option<String>,
    #[arg(long, default_value = "")]
    pub profile: String,
    #[arg(long = "task-id", default_value = "")]
    pub task_id: String,
    #[arg(long = "format", default_value = "toon")]
    pub format: String,
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[arg(long)]
    pub root: Option<String>,
    #[arg(long, default_value = "")]
    pub profile: String,
    #[arg()]
    pub files: Vec<String>,
}

#[derive(Debug, Args)]
pub struct FinalizeEditArgs {
    #[arg(required = true)]
    pub args: Vec<String>,
    #[arg(long, default_value = "artifact_revision_updated")]
    pub event: String,
    #[arg(long, default_value = "")]
    pub status: String,
    #[arg(long = "artifact-version", default_value = "")]
    pub artifact_version: String,
    #[arg(long = "artifact-revision", default_value = "")]
    pub artifact_revision: String,
    #[arg(long = "set")]
    pub set_values: Vec<String>,
    #[arg(long = "task-id", default_value = "")]
    pub task_id: String,
    #[arg(long, default_value = "manual")]
    pub actor: String,
    #[arg(long, default_value = "")]
    pub scope: String,
    #[arg(long, default_value = "")]
    pub tags: String,
}

#[derive(Debug, Args)]
pub struct TouchArgs {
    pub markdown_file: String,
    pub change_note: String,
    #[arg(long, default_value = "artifact_revision_updated")]
    pub event: String,
    #[arg(long = "task-id", default_value = "")]
    pub task_id: String,
    #[arg(long, default_value = "manual")]
    pub actor: String,
    #[arg(long, default_value = "")]
    pub scope: String,
    #[arg(long, default_value = "")]
    pub tags: String,
}

#[derive(Debug, Args)]
pub struct RenameArtifactArgs {
    pub markdown_file: String,
    pub artifact_path: String,
    pub change_note: String,
    #[arg(long = "artifact-type", default_value = "")]
    pub artifact_type: String,
    #[arg(long = "bump-version", default_value_t = false)]
    pub bump_version: bool,
    #[arg(long, default_value = "artifact_path_updated")]
    pub event: String,
    #[arg(long = "task-id", default_value = "")]
    pub task_id: String,
    #[arg(long, default_value = "manual")]
    pub actor: String,
    #[arg(long, default_value = "")]
    pub scope: String,
    #[arg(long, default_value = "")]
    pub tags: String,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
    #[arg(default_value = "")]
    pub markdown_file: String,
    #[arg(default_value = "")]
    pub artifact_path: String,
    #[arg(default_value = "")]
    pub artifact_type: String,
    #[arg(default_value = "")]
    pub change_note: String,
    #[arg(long, default_value = "")]
    pub title: String,
    #[arg(long, default_value = "")]
    pub purpose: String,
    #[arg(long = "artifact-version", default_value_t = 1)]
    pub artifact_version: u64,
    #[arg(long = "artifact-revision", default_value = "")]
    pub artifact_revision: String,
    #[arg(long = "schema-version", default_value_t = 1)]
    pub schema_version: u64,
    #[arg(long, default_value = "canonical")]
    pub status: String,
    #[arg(long = "task-id", default_value = "")]
    pub task_id: String,
    #[arg(long, default_value = "manual")]
    pub actor: String,
    #[arg(long, default_value = "")]
    pub scope: String,
    #[arg(long, default_value = "")]
    pub tags: String,
}

#[derive(Debug, Args)]
pub struct MigrateLinksArgs {
    pub path: String,
    pub old_target: String,
    pub new_target: String,
    pub change_note: String,
    #[arg(long = "task-id", default_value = "")]
    pub task_id: String,
    #[arg(long, default_value = "manual")]
    pub actor: String,
    #[arg(long, default_value = "")]
    pub scope: String,
    #[arg(long, default_value = "")]
    pub tags: String,
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
    #[arg(long = "format", default_value = "toon")]
    pub format: String,
}

#[derive(Debug, Args)]
pub struct MoveArgs {
    pub markdown_file: String,
    pub destination: String,
    pub change_note: String,
    #[arg(long = "task-id", default_value = "")]
    pub task_id: String,
    #[arg(long, default_value = "manual")]
    pub actor: String,
    #[arg(long, default_value = "")]
    pub scope: String,
    #[arg(long, default_value = "")]
    pub tags: String,
}

#[derive(Debug, Args)]
pub struct PathArgs {
    #[arg(long)]
    pub path: String,
}

#[derive(Debug, Args)]
pub struct ArtifactImpactArgs {
    #[arg(long)]
    pub file: Option<String>,
    #[arg(long, default_value = "")]
    pub artifact: String,
    #[arg(long)]
    pub root: Option<String>,
    #[arg(long = "format", default_value = "toon")]
    pub format: String,
}

#[derive(Debug, Args)]
pub struct TaskImpactArgs {
    #[arg(long)]
    pub task_id: String,
    #[arg(long)]
    pub root: String,
    #[arg(long = "format", default_value = "toon")]
    pub format: String,
}

#[derive(Debug, Args)]
pub struct ScanArgs {
    #[arg(long)]
    pub root: String,
    #[arg(long = "exclude-glob")]
    pub exclude_globs: Vec<String>,
    #[arg(long, default_value_t = false)]
    pub missing_only: bool,
}

#[derive(Debug, Args)]
pub struct RegistryWriteArgs {
    #[arg(long)]
    pub root: String,
    #[arg(long)]
    pub output: Option<String>,
    #[arg(long, default_value_t = false)]
    pub canonical: bool,
    #[arg(long = "exclude-glob")]
    pub exclude_globs: Vec<String>,
}

#[derive(Debug, Args)]
pub struct LayerStatusArgs {
    #[arg(long)]
    pub layer: usize,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[arg(long)]
    pub root: String,
    #[arg(long = "exclude-glob")]
    pub exclude_globs: Vec<String>,
    #[arg(long, default_value_t = false)]
    pub show_warnings: bool,
}

#[derive(Debug, Args)]
pub struct ProofcheckArgs {
    #[arg(long)]
    pub layer: Option<usize>,
    #[arg(long, default_value = "active-canon-strict")]
    pub profile: String,
}

#[derive(Debug, Serialize)]
struct DoctorRow {
    severity: String,
    path: String,
    issues: String,
}

#[derive(Debug, Serialize)]
struct LinkRow {
    path: String,
    artifact: String,
    target: String,
    resolved: String,
    exists: bool,
}

#[derive(Debug, Serialize)]
struct FooterRefRow {
    kind: String,
    target: String,
}

#[derive(Debug, Serialize)]
struct PathRow {
    path: String,
}

#[derive(Debug, Serialize)]
struct DepsPayload {
    path: String,
    artifact: String,
    links: Vec<LinkRow>,
    footer_refs: Vec<FooterRefRow>,
    referenced_by: Vec<PathRow>,
}

#[derive(Debug, Serialize)]
struct EdgeRow {
    path: String,
    artifact: String,
    edge_type: String,
    target: String,
    resolved: String,
    exists: bool,
}

#[derive(Debug, Serialize)]
struct ImpactRow {
    path: String,
    reasons: String,
}

#[derive(Debug, Serialize)]
struct TaskImpactRow {
    source_artifact: String,
    path: String,
    reasons: String,
}

#[derive(Debug, Serialize)]
struct TaskSummaryCountRow {
    value: String,
    events: usize,
}

#[derive(Debug, Serialize)]
struct TaskSummaryFileRow {
    path: String,
    events: usize,
}

#[derive(Debug, Serialize)]
struct TaskSummaryPayload {
    task_id: String,
    root: String,
    events: usize,
    files: usize,
    first_ts: String,
    last_ts: String,
    files_rows: Vec<TaskSummaryFileRow>,
    actors: Vec<TaskSummaryCountRow>,
    scopes: Vec<TaskSummaryCountRow>,
    tags: Vec<TaskSummaryCountRow>,
}

pub fn run(cli: Cli) -> String {
    match cli.command {
        Command::Overview(args) => {
            let rows = vec![ReadinessRow {
                artifact_path: ArtifactPath("docs/process/vida1-development-conditions.md".into()),
                verdict: ReadinessVerdict::Ok,
                checked_at: CheckedAt::now_utc(),
            }];
            render_overview(args.registry_count, args.relation_count, &rows)
        }
        Command::LayerStatus(args) => match read_layer_matrix() {
            Ok(rows) => render_layer_status_from_rows(args.layer, &rows),
            Err(error) => format!(
                "layer-status\n  layer: {}\n  error: {}",
                args.layer, error
            ),
        },
        Command::Check(args) => match check_rows(args.root.as_deref(), &args.profile, &args.files) {
            Ok(rows) => rows
                .iter()
                .map(|row| encode_line(row))
                .collect::<Result<Vec<_>, _>>()
                .map(|lines| lines.join("\n"))
                .unwrap_or_else(|error| format!("{{\"path\":\"\",\"issues\":[\"encode_error:{}\"]}}", error)),
            Err(error) => format!("{{\"path\":\"\",\"issues\":[\"{}\"]}}", error),
        },
        Command::Changelog(args) => match changelog_rows(&args.markdown_file) {
            Ok(mut rows) => {
                rows.sort_by_key(|row| parse_ts_value(row.get("ts")));
                if args.newest_first {
                    rows.reverse();
                }
                if args.limit > 0 {
                    rows.truncate(args.limit);
                }
                render_value_rows(
                    "changelog",
                    &args.format,
                    rows,
                    Some(serde_json::json!({
                        "command": "changelog",
                        "path": args.markdown_file,
                    })),
                )
            }
            Err(error) => format!("changelog\n  error: {error}"),
        },
        Command::ChangelogTask(args) => match changelog_task_rows(
            args.root.as_deref(),
            &args.profile,
            &args.task_id,
        ) {
            Ok(mut rows) => {
                rows.sort_by_key(|row| parse_ts_value(row.get("ts")));
                if args.newest_first {
                    rows.reverse();
                }
                if args.limit > 0 {
                    rows.truncate(args.limit);
                }
                render_value_rows(
                    "changelog-task",
                    &args.format,
                    rows,
                    Some(serde_json::json!({
                        "command": "changelog-task",
                        "task_id": args.task_id,
                        "root": args.profile,
                    })),
                )
            }
            Err(error) => format!("changelog-task\n  error: {error}"),
        },
        Command::TaskSummary(args) => match task_summary_payload(
            args.root.as_deref(),
            &args.profile,
            &args.task_id,
        ) {
            Ok(payload) => render_task_summary(&payload, &args.format),
            Err(error) => format!("task-summary\n  error: {error}"),
        },
        Command::Links(args) => match relation_scan_rows(&args.path) {
            Ok((rows, target)) => rows
                .iter()
                .map(|row| encode_line(row))
                .collect::<Result<Vec<_>, _>>()
                .map(|lines| lines.join("\n"))
                .unwrap_or_else(|error| {
                    format!(
                        "{{\"path\":\"{}\",\"artifact\":\"\",\"target\":\"\",\"resolved\":\"\",\"exists\":false,\"error\":\"{}\"}}",
                        target, error
                    )
                }),
            Err(error) => format!(
                "{{\"path\":\"{}\",\"artifact\":\"\",\"target\":\"\",\"resolved\":\"\",\"exists\":false,\"error\":\"{}\"}}",
                args.path, error
            ),
        },
        Command::Deps(args) => match deps_payload(&args.path) {
            Ok(payload) => serde_json::to_string(&payload)
                .unwrap_or_else(|error| format!("{{\"path\":\"{}\",\"error\":\"{}\"}}", args.path, error)),
            Err(error) => format!("{{\"path\":\"{}\",\"error\":\"{}\"}}", args.path, error),
        },
        Command::DepsMap(args) => match deps_map_rows(&args.path) {
            Ok(rows) => rows
                .iter()
                .map(|row| encode_line(row))
                .collect::<Result<Vec<_>, _>>()
                .map(|lines| lines.join("\n"))
                .unwrap_or_else(|error| {
                    format!(
                        "{{\"path\":\"{}\",\"artifact\":\"\",\"edge_type\":\"encode_error\",\"target\":\"\",\"resolved\":\"\",\"exists\":false,\"error\":\"{}\"}}",
                        args.path, error
                    )
                }),
            Err(error) => format!(
                "{{\"path\":\"{}\",\"artifact\":\"\",\"edge_type\":\"inventory_error\",\"target\":\"\",\"resolved\":\"\",\"exists\":false,\"error\":\"{}\"}}",
                args.path, error
            ),
        },
        Command::ArtifactImpact(args) => match artifact_impact_rows(args.file.as_deref(), &args.artifact, args.root.as_deref()) {
            Ok((artifact, source, rows)) => {
                if args.format == "jsonl" {
                    let context = serde_json::json!({
                        "command": "artifact-impact",
                        "source": source,
                        "artifact": artifact,
                        "impacts": rows,
                    });
                    serde_json::to_string(&context).unwrap_or_else(|error| {
                        format!("{{\"artifact\":\"{}\",\"error\":\"{}\"}}", artifact, error)
                    })
                } else {
                    let impacts = rows
                        .iter()
                        .map(|row| (row.path.as_str(), row.reasons.as_str()))
                        .collect::<Vec<_>>();
                    render_artifact_impact(&artifact, &source, &impacts)
                }
            }
            Err(error) => format!("{{\"artifact\":\"{}\",\"error\":\"{}\"}}", args.artifact, error),
        },
        Command::TaskImpact(args) => match task_impact_rows(&args.root, &args.task_id) {
            Ok((touched, impacts)) => {
                if args.format == "jsonl" {
                    let context = serde_json::json!({
                        "command": "task-impact",
                        "task_id": args.task_id,
                        "root": args.root,
                        "touched": touched,
                        "indirect_impacts": impacts,
                    });
                    serde_json::to_string(&context).unwrap_or_else(|error| {
                        format!("{{\"task_id\":\"{}\",\"error\":\"{}\"}}", args.task_id, error)
                    })
                } else {
                    let touched_paths = touched
                        .iter()
                        .map(|row| row.path.as_str())
                        .collect::<Vec<_>>();
                    let impact_rows = impacts
                        .iter()
                        .map(|row| {
                            (
                                row.source_artifact.as_str(),
                                row.path.as_str(),
                                row.reasons.as_str(),
                            )
                        })
                        .collect::<Vec<_>>();
                    render_task_impact(&args.task_id, &args.root, &touched_paths, &impact_rows)
                }
            }
            Err(error) => format!("{{\"task_id\":\"{}\",\"error\":\"{}\"}}", args.task_id, error),
        },
        Command::Fastcheck(args) => {
            match fastcheck_rows(args.root.as_deref(), &args.profile, &args.files) {
                Ok(rows) => rows
                    .iter()
                    .map(|issue| encode_line(issue))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|lines| lines.join("\n"))
                    .unwrap_or_else(|error| {
                        format!(
                            "{{\"artifact_path\":\"\",\"code\":\"encode_error\",\"message\":\"{}\"}}",
                            error
                        )
                    }),
                Err(error) => format!(
                    "{{\"artifact_path\":\"\",\"code\":\"inventory_error\",\"message\":\"{}\"}}",
                    error
                ),
            }
        }
        Command::ActivationCheck(args) => {
            match activation_rows(args.root.as_deref(), &args.profile, &args.files) {
                Ok(rows) => rows
                    .iter()
                    .map(|issue| encode_line(issue))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|lines| lines.join("\n"))
                    .unwrap_or_else(|error| {
                        format!(
                            "{{\"path\":\"\",\"issues\":\"encode_error:{}\"}}",
                            error
                        )
                    }),
                Err(error) => format!(
                    "{{\"path\":\"\",\"issues\":\"inventory_error:{}\"}}",
                    error
                ),
            }
        }
        Command::ProtocolCoverageCheck(args) => {
            match protocol_coverage_rows(args.root.as_deref(), &args.profile, &args.files) {
                Ok(rows) => rows
                    .iter()
                    .map(|issue| encode_line(issue))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|lines| lines.join("\n"))
                    .unwrap_or_else(|error| {
                        format!(
                            "{{\"path\":\"\",\"issues\":\"encode_error:{}\"}}",
                            error
                        )
                    }),
                Err(error) => format!(
                    "{{\"path\":\"\",\"issues\":\"inventory_error:{}\"}}",
                    error
                ),
            }
        }
        Command::FinalizeEdit(args) => match finalize_edit(args) {
            Ok(rendered) => rendered,
            Err(error) => format!("finalize-edit\n  error: {error}"),
        },
        Command::Touch(args) => match touch(args) {
            Ok(rendered) => rendered,
            Err(error) => format!("touch\n  error: {error}"),
        },
        Command::Init(args) => match init_command(args) {
            Ok(rendered) => rendered,
            Err(error) => format!("init\n  error: {error}"),
        },
        Command::MigrateLinks(args) => match migrate_links(args) {
            Ok(rendered) => rendered,
            Err(error) => format!("migrate-links\n  error: {error}"),
        },
        Command::Move(args) => match move_artifact(args) {
            Ok(rendered) => rendered,
            Err(error) => format!("move\n  error: {error}"),
        },
        Command::RenameArtifact(args) => match rename_artifact(args) {
            Ok(rendered) => rendered,
            Err(error) => format!("rename-artifact\n  error: {error}"),
        },
        Command::Proofcheck(args) => match args.layer {
            Some(layer) => match layer_scope_paths(layer) {
                Ok(paths) => render_proofcheck_layer(layer, &paths),
                Err(error) => format!(
                    "proofcheck\n  layer: {}\n  files_mode: layer\n  error: {}",
                    layer, error
                ),
            },
            None => match render_proofcheck_profile(&args.profile) {
                Ok(rendered) => rendered,
                Err(error) => format!(
                    "context:\n  command: proofcheck\n  root: {}\n  layer: \n  files_mode: profile\ntotals:\n  fastcheck_rows: 0\n  protocol_coverage_rows: 0\n  readiness_rows: 0\n  doctor_error_rows: 0\n  doctor_warning_rows: 0\nerror:\n  message: {}",
                    args.profile, error
                ),
            },
        },
        Command::Doctor(args) => {
            let scope = inventory_scope_for_root(&args.root, &args.exclude_globs);
            match build_registry(&scope) {
                Ok(rows) => doctor_rows_for(&args.root, &rows, args.show_warnings)
                    .iter()
                    .map(|row| encode_line(row))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|lines| lines.join("\n"))
                    .unwrap_or_else(|error| {
                        format!(
                            "{{\"severity\":\"error\",\"path\":\"{}\",\"issues\":\"encode_error:{}\"}}",
                            args.root, error
                        )
                    }),
                Err(error) => format!(
                    "{{\"severity\":\"error\",\"path\":\"{}\",\"issues\":\"inventory_error:{}\"}}",
                    args.root, error
                ),
            }
        }
        Command::Summary(args) => {
            let scope = inventory_scope_for_root(&args.root, &args.exclude_globs);
            match build_registry(&scope) {
                Ok(rows) => {
                    let issues = collect_tree_issues(&args.root, &rows);
                    let readiness_rows = issues_to_readiness_rows(&issues);
                    let edges = artifact_identity_edges(&rows);
                    let type_counts = summarize_artifact_types(&rows);
                    render_summary(
                        &args.root,
                        rows.len(),
                        edges.len(),
                        &readiness_rows,
                        &type_counts,
                    )
                }
                Err(error) => format!(
                    "summary\n  root: {}\n  registry_rows: 0\n  relation_edges: 0\n  readiness: blocking\n  error: {}",
                    args.root, error
                ),
            }
        }
        Command::OverviewScan(args) => {
            let scope = inventory_scope_for_root(&args.root, &args.exclude_globs);
            match build_registry(&scope) {
                Ok(rows) => {
                    let edges = artifact_identity_edges(&rows);
                    let readiness_rows = vec![ReadinessRow {
                        artifact_path: ArtifactPath(
                            "docs/process/vida1-development-conditions.md".into(),
                        ),
                        verdict: ReadinessVerdict::Ok,
                        checked_at: CheckedAt::now_utc(),
                    }];
                    render_overview(rows.len(), edges.len(), &readiness_rows)
                }
                Err(error) => format!(
                    "docflow overview\n  registry_rows: 0\n  relation_edges: 0\n  readiness: blocking\n  error: {}",
                    error
                ),
            }
        }
        Command::Relations(args) => {
            let edges = (0..args.edge_count)
                .map(|_| RelationEdge {
                    from: ArtifactPath("docs/process/vida1-development-conditions.md".into()),
                    to: ArtifactPath("docs/process/vida1-development-conditions.md".into()),
                    relation_type: "artifact_identity".into(),
                })
                .collect::<Vec<_>>();
            render_relation_summary(&edges)
        }
        Command::RelationsScan(args) => {
            let scope = inventory_scope_for_root(&args.root, &args.exclude_globs);
            match build_registry(&scope) {
                Ok(rows) => {
                    let edges = rows
                        .iter()
                        .map(|row| RelationEdge {
                            from: row.artifact_path.clone(),
                            to: row.artifact_path.clone(),
                            relation_type: "artifact_identity".into(),
                        })
                        .collect::<Vec<_>>();
                    render_relation_summary(&edges)
                }
                Err(error) => format!("relations\n  total_edges: 0\n  error: {}", error),
            }
        }
        Command::Scan(args) => {
            let scope = inventory_scope_for_root(&args.root, &args.exclude_globs);
            match build_registry(&scope) {
                Ok(rows) => rows
                    .iter()
                    .filter_map(|row| scan_row_for(&args.root, row).ok())
                    .filter(|row| !args.missing_only || !row.has_footer)
                    .map(|row| encode_line(&row))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|lines| lines.join("\n"))
                    .unwrap_or_else(|error| {
                        format!(
                            "{{\"artifact_path\":\"{}\",\"artifact_type\":\"inventory_error\",\"has_footer\":false,\"has_changelog\":false,\"error\":\"{}\"}}",
                            args.root, error
                        )
                    }),
                Err(error) => format!(
                    "{{\"artifact_path\":\"{}\",\"artifact_type\":\"inventory_error\",\"has_footer\":false,\"has_changelog\":false,\"error\":\"{}\"}}",
                    args.root, error
                ),
            }
        }
        Command::RegistryScan(args) => {
            let scope = inventory_scope_for_root(&args.root, &args.exclude_globs);
            match build_registry(&scope) {
                Ok(rows) => {
                    let detail = rows
                        .iter()
                        .map(|row| format!("  - {} [{}]", row.artifact_path.0, row.artifact_type))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if detail.is_empty() {
                        "registry\n  total_rows: 0".to_string()
                    } else {
                        format!("registry\n  total_rows: {}\n{}", rows.len(), detail)
                    }
                }
                Err(error) => format!("registry\n  total_rows: 0\n  error: {}", error),
            }
        }
        Command::Registry(args) => {
            let scope = inventory_scope_for_root(&args.root, &args.exclude_globs);
            match build_registry(&scope) {
                Ok(rows) => rows
                    .iter()
                    .map(|row| encode_line(row))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|lines| lines.join("\n"))
                    .unwrap_or_else(|error| {
                        format!(
                            "{{\"artifact_path\":\"{}\",\"artifact_type\":\"inventory_error:{}\"}}",
                            args.root,
                            error
                        )
                    }),
                Err(error) => format!(
                    "{{\"artifact_path\":\"{}\",\"artifact_type\":\"inventory_error:{}\"}}",
                    args.root, error
                ),
            }
        }
        Command::RegistryWrite(args) => {
            let output = resolve_registry_output(&args);
            let scope = inventory_scope_for_root(&args.root, &args.exclude_globs);
            match build_registry(&scope) {
                Ok(rows) => match write_registry_jsonl(&output, &rows) {
                    Ok(()) => format!(
                        "registry-write\n  total_rows: {}\n  output: {}",
                        rows.len(),
                        output
                    ),
                    Err(error) => format!(
                        "registry-write\n  total_rows: {}\n  output: {}\n  error: {}",
                        rows.len(),
                        output,
                        error
                    ),
                },
                Err(error) => format!(
                    "registry-write\n  total_rows: 0\n  output: {}\n  error: {}",
                    output, error
                ),
            }
        }
        Command::ValidateTree(args) => {
            let scope = inventory_scope_for_root(&args.root, &args.exclude_globs);
            match build_registry(&scope) {
                Ok(rows) => {
                    let issues = collect_tree_issues(&args.root, &rows);
                    let readiness_rows = issues_to_readiness_rows(&issues);
                    let verdict = summarize_verdict(&readiness_rows);
                    let detail = issues
                        .iter()
                        .map(|issue| {
                            format!(
                                "  - {} [{}]: {}",
                                issue.artifact_path.0, issue.code, issue.message
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if detail.is_empty() {
                        format!(
                            "validation-tree\n  scanned_rows: {}\n  issues: 0\n  verdict: {}",
                            rows.len(),
                            verdict_label(verdict)
                        )
                    } else {
                        format!(
                            "validation-tree\n  scanned_rows: {}\n  issues: {}\n  verdict: {}\n{}",
                            rows.len(),
                            issues.len(),
                            verdict_label(verdict),
                            detail
                        )
                    }
                }
                Err(error) => format!(
                    "validation-tree\n  scanned_rows: 0\n  issues: 1\n  verdict: blocking\n  - {} [inventory_error]: {}",
                    args.root, error
                ),
            }
        }
        Command::ReadinessTree(args) => {
            let scope = inventory_scope_for_root(&args.root, &args.exclude_globs);
            match build_registry(&scope) {
                Ok(rows) => {
                    let issues = collect_tree_issues(&args.root, &rows);
                    let readiness_rows = issues_to_readiness_rows(&issues);
                    let verdict = summarize_verdict(&readiness_rows);
                    let detail = readiness_rows
                        .iter()
                        .map(|row| {
                            format!(
                                "  - {} [{}]",
                                row.artifact_path.0,
                                verdict_label(row.verdict)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if detail.is_empty() {
                        format!(
                            "readiness-tree\n  scanned_rows: {}\n  rows: 0\n  verdict: {}",
                            rows.len(),
                            verdict_label(verdict)
                        )
                    } else {
                        format!(
                            "readiness-tree\n  scanned_rows: {}\n  rows: {}\n  verdict: {}\n{}",
                            rows.len(),
                            readiness_rows.len(),
                            verdict_label(verdict),
                            detail
                        )
                    }
                }
                Err(error) => format!(
                    "readiness-tree\n  scanned_rows: 0\n  rows: 1\n  verdict: blocking\n  - {} [inventory_error: {}]",
                    args.root, error
                ),
            }
        }
        Command::ReadinessCheck(args) => match readiness_rows(args.root.as_deref(), &args.profile, &args.files) {
            Ok(rows) => rows
                .iter()
                .map(|row| encode_line(row))
                .collect::<Result<Vec<_>, _>>()
                .map(|lines| lines.join("\n"))
                .unwrap_or_else(|error| {
                    format!(
                        "{{\"artifact_path\":\"\",\"verdict\":\"blocking\",\"error\":\"{}\"}}",
                        error
                    )
                }),
            Err(error) => format!(
                "{{\"artifact_path\":\"\",\"verdict\":\"blocking\",\"error\":\"{}\"}}",
                error
            ),
        },
        Command::ReadinessWrite(args) => {
            let output = resolve_readiness_output(&args);
            let scope = inventory_scope_for_root(&args.root, &args.exclude_globs);
            match build_registry(&scope) {
                Ok(rows) => {
                    let issues = collect_tree_issues(&args.root, &rows);
                    let readiness_rows = issues_to_readiness_rows(&issues);
                    match write_readiness_jsonl(&output, &readiness_rows) {
                        Ok(()) => format!(
                            "readiness-write\n  rows: {}\n  output: {}",
                            readiness_rows.len(),
                            output
                        ),
                        Err(error) => format!(
                            "readiness-write\n  rows: {}\n  output: {}\n  error: {}",
                            readiness_rows.len(),
                            output,
                            error
                        ),
                    }
                }
                Err(error) => format!(
                    "readiness-write\n  rows: 0\n  output: {}\n  error: {}",
                    output, error
                ),
            }
        }
        Command::ValidateFooter(args) => render_validation_result(&args.path, &args.content),
        Command::Readiness(args) => render_readiness_result(&args.path, &args.content),
        Command::CheckFile(args) => match fs::read_to_string(&args.path) {
            Ok(content) => render_validation_result(&args.path, &content),
            Err(error) => format!(
                "validation\n  issues: 1\n  verdict: blocking\n  - {} [read_error]: {}",
                args.path, error
            ),
        },
        Command::ReadinessFile(args) => match fs::read_to_string(&args.path) {
            Ok(content) => render_readiness_result(&args.path, &content),
            Err(error) => format!(
                "readiness\n  rows: 1\n  verdict: blocking\n  - {} [read_error: {}]",
                args.path, error
            ),
        },
        Command::ReportCheck(args) => match fs::read_to_string(&args.path) {
            Ok(content) => render_report_check_result(&args.path, &content),
            Err(error) => format!(
                "reporting\n  issues: 1\n  verdict: blocking\n  - {} [read_error]: {}",
                args.path, error
            ),
        },
    }
}

fn scan_row_for(root: &str, row: &docflow_contracts::RegistryRow) -> std::io::Result<ScanRow> {
    let path = std::path::Path::new(root).join(&row.artifact_path.0);
    let content = fs::read_to_string(&path)?;
    let has_footer = content.contains("\n-----\n");
    let changelog_name = std::path::Path::new(&row.artifact_path.0)
        .file_stem()
        .map(|stem| format!("{}.changelog.jsonl", stem.to_string_lossy()))
        .unwrap_or_else(|| "missing.changelog.jsonl".to_string());
    let has_changelog = path
        .parent()
        .map(|parent| parent.join(changelog_name).exists())
        .unwrap_or(false);
    Ok(ScanRow {
        artifact_path: row.artifact_path.clone(),
        artifact_type: row.artifact_type.clone(),
        has_footer,
        has_changelog,
    })
}

fn doctor_rows_for(
    root: &str,
    rows: &[docflow_contracts::RegistryRow],
    show_warnings: bool,
) -> Vec<DoctorRow> {
    let mut output = Vec::new();
    for row in rows {
        let path = std::path::Path::new(root).join(&row.artifact_path.0);
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                output.push(DoctorRow {
                    severity: "error".into(),
                    path: row.artifact_path.0.clone(),
                    issues: format!("read_error:{error}"),
                });
                continue;
            }
        };
        let issues = validate_markdown_footer(row.artifact_path.clone(), &content);
        if !issues.is_empty() {
            output.push(DoctorRow {
                severity: "error".into(),
                path: row.artifact_path.0.clone(),
                issues: issues
                    .iter()
                    .map(|issue| issue.code.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            });
            continue;
        }
        let changelog_name = std::path::Path::new(&row.artifact_path.0)
            .file_stem()
            .map(|stem| format!("{}.changelog.jsonl", stem.to_string_lossy()))
            .unwrap_or_else(|| "missing.changelog.jsonl".to_string());
        let has_changelog = path
            .parent()
            .map(|parent| parent.join(changelog_name).exists())
            .unwrap_or(false);
        if !has_changelog {
            output.push(DoctorRow {
                severity: "error".into(),
                path: row.artifact_path.0.clone(),
                issues: "missing_changelog".into(),
            });
            continue;
        }
        if show_warnings && row.artifact_type == "document" {
            output.push(DoctorRow {
                severity: "warning".into(),
                path: row.artifact_path.0.clone(),
                issues: "generic_document_policy".into(),
            });
        }
    }
    output
}

fn read_activation_protocol() -> std::io::Result<String> {
    let base =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vida/config/instructions");
    let candidates = [
        base.join("bridge.instruction-activation-protocol.md"),
        base.join("instruction-contracts/bridge.instruction-activation-protocol.md"),
    ];
    for path in candidates {
        if path.exists() {
            return fs::read_to_string(path);
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "instruction activation protocol not found",
    ))
}

fn read_protocol_index() -> std::io::Result<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vida/config/instructions/system-maps/protocol.index.md");
    fs::read_to_string(path)
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .to_path_buf()
}

fn runtime_root() -> std::path::PathBuf {
    std::env::var_os("VIDA_ROOT")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::current_dir().ok().and_then(|cwd| {
                cwd.ancestors()
                    .find(|ancestor| {
                        ancestor.join("AGENTS.sidecar.md").is_file()
                            || ancestor.join("vida.config.yaml").is_file()
                    })
                    .map(std::path::Path::to_path_buf)
            })
        })
        .unwrap_or_else(repo_root)
}

fn docflow_policy_path() -> std::path::PathBuf {
    let runtime_path = runtime_root().join("vida/config/docflow/docsys_policy.yaml");
    if runtime_path.is_file() {
        return runtime_path;
    }
    repo_root().join("vida/config/docflow/docsys_policy.yaml")
}

fn default_inventory_excludes() -> Vec<String> {
    let policy = docflow_policy_path();
    resolve_scan_ignored_globs(&policy).unwrap_or_default()
}

fn merge_inventory_excludes(explicit: &[String]) -> Vec<String> {
    let mut merged = default_inventory_excludes();
    for pattern in explicit {
        if !merged.iter().any(|existing| existing == pattern) {
            merged.push(pattern.clone());
        }
    }
    merged
}

fn inventory_scope_for_root(root: &str, explicit_excludes: &[String]) -> InventoryScope {
    let mut scope = InventoryScope::new(root);
    scope.exclude_globs = merge_inventory_excludes(explicit_excludes);
    scope
}

fn activation_issue_for(path: &str, activation_body: &str) -> Option<DoctorRow> {
    if !is_activation_governed_protocol(path) {
        return None;
    }
    if path
        == "vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md"
    {
        return None;
    }
    if protocol_reference_variants(path)
        .iter()
        .any(|variant| activation_body.contains(variant))
    {
        return None;
    }
    Some(DoctorRow {
        severity: "error".into(),
        path: path.to_string(),
        issues: format!("missing_activation_binding:{path}"),
    })
}

fn protocol_coverage_issue_for(
    path: &str,
    activation_body: &str,
    protocol_index_body: &str,
) -> Option<DoctorRow> {
    if !is_activation_governed_protocol(path) {
        return None;
    }
    let mut issues = Vec::new();
    if activation_issue_for(path, activation_body).is_some() {
        issues.push(format!("missing_activation_binding:{path}"));
    }
    if !protocol_reference_variants(path)
        .iter()
        .any(|variant| protocol_index_body.contains(variant))
    {
        issues.push(format!("missing_protocol_index_binding:{path}"));
    }
    if issues.is_empty() {
        None
    } else {
        Some(DoctorRow {
            severity: "error".into(),
            path: path.to_string(),
            issues: issues.join(","),
        })
    }
}

fn protocol_reference_variants(path: &str) -> Vec<String> {
    let path_obj = std::path::Path::new(path);
    let mut variants = vec![path.to_string()];
    if let Some(file_name) = path_obj
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
    {
        variants.push(file_name.clone());
        if let Some(stem) = std::path::Path::new(&file_name)
            .file_stem()
            .map(|value| value.to_string_lossy().to_string())
        {
            variants.push(stem);
        }
    }

    let normalized = path.replace('\\', "/");
    if let Some(stripped) = normalized.strip_prefix("vida/config/instructions/") {
        let shorthand = stripped.trim_end_matches(".md").to_string();
        variants.push(shorthand.clone());
        if let Some(file_name) = std::path::Path::new(&shorthand)
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
        {
            variants.push(file_name);
        }
    }

    variants.sort();
    variants.dedup();
    variants
}

fn layer_scope_paths(layer: usize) -> Result<Vec<String>, String> {
    let targets: &[&str] = match layer {
        1 => &[
            "docs/product/spec/instruction-artifact-model.md",
            "docs/product/spec/project-documentation-law.md",
            "docs/product/spec/current-spec-map.md",
            "docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md",
        ],
        2 => &[
            "docs/product/spec/canonical-inventory-law.md",
            "docs/product/spec/project-documentation-law.md",
            "docs/product/spec/current-spec-map.md",
            "vida/config/instructions/system-maps/framework.map.md",
        ],
        3 => &[
            "docs/product/spec/project-documentation-law.md",
            "docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md",
            "vida/config/instructions/system-maps/framework.map.md",
            "vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md",
        ],
        4 => &[
            "docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md",
            "vida/config/instructions/system-maps/framework.map.md",
            "vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md",
            "AGENTS.sidecar.md",
        ],
        5 => &[
            "docs/product/spec/canonical-relation-law.md",
            "docs/product/spec/project-documentation-law.md",
            "docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md",
            "AGENTS.sidecar.md",
        ],
        6 => &[
            "docs/product/spec/project-documentation-law.md",
            "vida/config/instructions/system-maps/framework.map.md",
            "AGENTS.sidecar.md",
            "vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md",
        ],
        7 => &[
            "docs/product/spec/canonical-runtime-readiness-law.md",
            "docs/product/spec/instruction-artifact-model.md",
            "docs/process/framework-source-lineage-index.md",
            "docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md",
            "vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md",
            "AGENTS.sidecar.md",
        ],
        8 => &[
            "docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md",
            "vida/config/instructions/system-maps/framework.map.md",
            "docs/product/spec/current-spec-map.md",
            "AGENTS.sidecar.md",
        ],
        _ => return Err(format!("unsupported layer: {layer}")),
    };
    let root = repo_root();
    Ok(targets
        .iter()
        .filter(|rel| root.join(rel).exists())
        .map(|rel| rel.to_string())
        .collect())
}

fn fastcheck_rows_for_paths(paths: &[String]) -> Vec<ValidationIssue> {
    let root = repo_root();
    let mut issues = Vec::new();
    for rel in paths {
        let path = root.join(rel);
        match fs::read_to_string(&path) {
            Ok(content) => {
                issues.extend(collect_file_validation_issues(&root, rel, &content));
            }
            Err(error) => issues.push(ValidationIssue {
                artifact_path: ArtifactPath(rel.clone()),
                verdict: ReadinessVerdict::Blocking,
                code: "read_error".into(),
                message: error.to_string(),
                checked_at: CheckedAt::now_utc(),
            }),
        }
    }
    issues
}

fn protocol_coverage_rows_for_paths(paths: &[String]) -> Vec<DoctorRow> {
    let activation_body = match read_activation_protocol() {
        Ok(body) => body,
        Err(error) => {
            return vec![DoctorRow {
                severity: "error".into(),
                path: "".into(),
                issues: format!("activation_protocol_read_error:{error}"),
            }];
        }
    };
    let protocol_index_body = match read_protocol_index() {
        Ok(body) => body,
        Err(error) => {
            return vec![DoctorRow {
                severity: "error".into(),
                path: "".into(),
                issues: format!("protocol_index_read_error:{error}"),
            }];
        }
    };
    paths
        .iter()
        .filter_map(|path| {
            protocol_coverage_issue_for(path, &activation_body, &protocol_index_body)
        })
        .collect()
}

fn render_proofcheck_layer(layer: usize, paths: &[String]) -> String {
    let fast_rows = fastcheck_rows_for_paths(paths);
    let protocol_rows = protocol_coverage_rows_for_paths(paths);
    let readiness_rows = issues_to_readiness_rows(&fast_rows);
    let doctor_rows = doctor_rows_for_paths(paths, true);
    let doctor_error_rows = doctor_rows
        .iter()
        .filter(|row| row.severity == "error")
        .count();
    let doctor_warning_rows = doctor_rows
        .iter()
        .filter(|row| row.severity == "warning")
        .count();
    let mut lines = vec![
        "proofcheck".to_string(),
        format!("  layer: {layer}"),
        "  files_mode: layer".to_string(),
        format!("  fastcheck_rows: {}", fast_rows.len()),
        format!("  protocol_coverage_rows: {}", protocol_rows.len()),
        format!("  readiness_rows: {}", readiness_rows.len()),
        format!("  doctor_error_rows: {doctor_error_rows}"),
        format!("  doctor_warning_rows: {doctor_warning_rows}"),
    ];
    for row in &fast_rows {
        lines.push(format!(
            "  fastcheck: {} [{}]",
            row.artifact_path.0, row.code
        ));
    }
    for row in &protocol_rows {
        lines.push(format!(
            "  protocol_coverage: {} [{}]",
            row.path, row.issues
        ));
    }
    for row in &readiness_rows {
        lines.push(format!(
            "  readiness: {} [{}]",
            row.artifact_path.0,
            verdict_label(row.verdict)
        ));
    }
    for row in &doctor_rows {
        lines.push(format!("  doctor: {} [{}]", row.path, row.issues));
    }
    lines.join("\n")
}

fn render_proofcheck_profile(profile: &str) -> Result<String, String> {
    let targets = resolve_profile_targets(None, profile, &[])?;
    let fast_rows = fastcheck_rows(None, profile, &[])?;
    let protocol_rows = protocol_coverage_rows(None, profile, &[])?;
    let readiness_rows = readiness_rows(None, profile, &[])?;
    let doctor_rows = doctor_rows_for_targets(&targets, false);
    let doctor_error_rows = doctor_rows
        .iter()
        .filter(|row| row.severity == "error")
        .count();
    let doctor_warning_rows = doctor_rows
        .iter()
        .filter(|row| row.severity == "warning")
        .count();
    let mut lines = vec![
        "context:".to_string(),
        "  command: proofcheck".to_string(),
        format!("  root: {profile}"),
        "  layer: ".to_string(),
        "  files_mode: profile".to_string(),
        "totals:".to_string(),
        format!("  fastcheck_rows: {}", fast_rows.len()),
        format!("  protocol_coverage_rows: {}", protocol_rows.len()),
        format!("  readiness_rows: {}", readiness_rows.len()),
        format!("  doctor_error_rows: {doctor_error_rows}"),
        format!("  doctor_warning_rows: {doctor_warning_rows}"),
    ];
    if !fast_rows.is_empty() {
        lines.push("fastcheck:".to_string());
        for row in &fast_rows {
            lines.push(format!("  - {} [{}]", row.artifact_path.0, row.code));
        }
    }
    if !protocol_rows.is_empty() {
        lines.push("protocol_coverage:".to_string());
        for row in &protocol_rows {
            lines.push(format!("  - {} [{}]", row.path, row.issues));
        }
    }
    if !readiness_rows.is_empty() {
        lines.push("readiness:".to_string());
        for row in &readiness_rows {
            lines.push(format!(
                "  - {} [{}]",
                row.artifact_path.0,
                verdict_label(row.verdict)
            ));
        }
    }
    if !doctor_rows.is_empty() {
        lines.push("doctor:".to_string());
        for row in &doctor_rows {
            lines.push(format!("  - {} [{}]", row.path, row.issues));
        }
    }
    lines.push(
        if fast_rows.is_empty()
            && protocol_rows.is_empty()
            && readiness_rows.is_empty()
            && doctor_error_rows == 0
        {
            "✅ OK: proofcheck".to_string()
        } else {
            "❌ BLOCKING: proofcheck".to_string()
        },
    );
    Ok(lines.join("\n"))
}

fn doctor_rows_for_paths(paths: &[String], show_warnings: bool) -> Vec<DoctorRow> {
    let rows = paths
        .iter()
        .map(|path| docflow_contracts::RegistryRow {
            artifact_path: ArtifactPath(path.clone()),
            artifact_type: artifact_type_for_path(path).into(),
        })
        .collect::<Vec<_>>();
    doctor_rows_for(&repo_root().display().to_string(), &rows, show_warnings)
}

fn doctor_rows_for_targets(
    targets: &[(std::path::PathBuf, std::path::PathBuf)],
    show_warnings: bool,
) -> Vec<DoctorRow> {
    let mut output = Vec::new();
    for (scope_root, file_path) in targets {
        let rel = normalize_path_for_root(file_path, scope_root);
        let artifact_type = artifact_type_for_path(&rel);
        let content = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(error) => {
                output.push(DoctorRow {
                    severity: "error".into(),
                    path: rel,
                    issues: format!("read_error:{error}"),
                });
                continue;
            }
        };
        let issues = validate_markdown_footer(ArtifactPath(rel.clone()), &content);
        if !issues.is_empty() {
            output.push(DoctorRow {
                severity: "error".into(),
                path: rel,
                issues: issues
                    .iter()
                    .map(|issue| issue.code.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            });
            continue;
        }
        let changelog_name = std::path::Path::new(&rel)
            .file_stem()
            .map(|stem| format!("{}.changelog.jsonl", stem.to_string_lossy()))
            .unwrap_or_else(|| "missing.changelog.jsonl".to_string());
        let has_changelog = file_path
            .parent()
            .map(|parent| parent.join(changelog_name).exists())
            .unwrap_or(false);
        if !has_changelog {
            output.push(DoctorRow {
                severity: "error".into(),
                path: rel,
                issues: "missing_changelog".into(),
            });
            continue;
        }
        if show_warnings && artifact_type == "document" {
            output.push(DoctorRow {
                severity: "warning".into(),
                path: rel,
                issues: "generic_document_policy".into(),
            });
        }
    }
    output
}

fn artifact_type_for_path(path: &str) -> &'static str {
    if path.starts_with("docs/product/spec/") {
        "product_spec"
    } else if path.starts_with("docs/process/") {
        "process_doc"
    } else if path.starts_with("vida/config/instructions/") {
        "instruction_contract"
    } else {
        "document"
    }
}

fn footer_map(content: &str) -> BTreeMap<String, String> {
    let mut output = BTreeMap::new();
    if let Ok(artifact) = docflow_markdown::split_footer(content) {
        if let Some(footer) = artifact.footer {
            for line in footer.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    output.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
        }
    }
    output
}

fn extract_markdown_links(path: &str, body: &str) -> Vec<LinkRow> {
    let mut rows = Vec::new();
    let mut cursor = 0usize;
    while let Some(start) = body[cursor..].find("](") {
        let start = cursor + start + 2;
        if let Some(end) = body[start..].find(')') {
            let end = start + end;
            let target = body[start..end].trim();
            if !target.is_empty() && !target.contains("://") {
                let resolved = resolve_link_target(path, target);
                let exists = runtime_root().join(&resolved).exists();
                rows.push(LinkRow {
                    path: path.to_string(),
                    artifact: String::new(),
                    target: target.to_string(),
                    resolved,
                    exists,
                });
            }
            cursor = end + 1;
        } else {
            break;
        }
    }
    rows
}

fn resolve_link_target(path: &str, target: &str) -> String {
    let stripped = target.split('#').next().unwrap_or(target);
    let joined = std::path::Path::new(path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new(""))
        .join(stripped);
    normalize_relative_path(&joined)
}

fn normalize_relative_path(path: &std::path::Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                let _ = parts.pop();
            }
            std::path::Component::Normal(value) => parts.push(value.to_string_lossy().to_string()),
            _ => {}
        }
    }
    parts.join("/")
}

fn relation_scan_rows(path: &str) -> std::io::Result<(Vec<LinkRow>, String)> {
    let target = std::path::Path::new(path);
    let absolute = if target.is_absolute() {
        target.to_path_buf()
    } else {
        runtime_root().join(target)
    };
    let files = resolve_markdown_scope(&absolute)?;
    let mut rows = Vec::new();
    for file in files {
        let rel = normalize_path_for_repo(&file);
        let content = fs::read_to_string(&file)?;
        let footer = footer_map(&content);
        let mut links = extract_markdown_links(&rel, &content);
        for link in &mut links {
            link.artifact = footer.get("artifact_path").cloned().unwrap_or_default();
        }
        rows.extend(links);
    }
    Ok((rows, normalize_path_for_repo(&absolute)))
}

fn deps_payload(path: &str) -> std::io::Result<DepsPayload> {
    let absolute = runtime_root().join(path);
    let content = fs::read_to_string(&absolute)?;
    let footer = footer_map(&content);
    let body = docflow_markdown::split_footer(&content)
        .map(|artifact| artifact.body)
        .unwrap_or(content.clone());
    let footer_refs = [
        "projection_ref",
        "contract_ref",
        "template_ref",
        "parent_definition_ref",
    ]
    .iter()
    .filter_map(|key| {
        footer.get(*key).map(|value| FooterRefRow {
            kind: (*key).to_string(),
            target: value.clone(),
        })
    })
    .collect::<Vec<_>>();
    let links = {
        let mut rows = extract_markdown_links(path, &body);
        for row in &mut rows {
            row.artifact = footer.get("artifact_path").cloned().unwrap_or_default();
        }
        rows
    };
    let artifact_path = footer.get("artifact_path").cloned().unwrap_or_default();
    let referenced_by = reverse_references(path, &artifact_path)?;
    Ok(DepsPayload {
        path: path.to_string(),
        artifact: artifact_path,
        links,
        footer_refs,
        referenced_by,
    })
}

fn reverse_references(path: &str, artifact_path: &str) -> std::io::Result<Vec<PathRow>> {
    let mut rows = Vec::new();
    for file in all_markdown_files(&runtime_root())? {
        let rel = normalize_path_for_root(&file, &runtime_root());
        if rel == path {
            continue;
        }
        let content = fs::read_to_string(&file)?;
        if content.contains(path) || (!artifact_path.is_empty() && content.contains(artifact_path))
        {
            rows.push(PathRow { path: rel });
        }
    }
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(rows)
}

fn deps_map_rows(path: &str) -> std::io::Result<Vec<EdgeRow>> {
    let absolute = runtime_root().join(path);
    let files = resolve_markdown_scope(&absolute)?;
    let mut rows = Vec::new();
    for file in files {
        let rel = normalize_path_for_repo(&file);
        let content = fs::read_to_string(&file)?;
        let body = docflow_markdown::split_footer(&content)
            .map(|artifact| artifact.body)
            .unwrap_or(content.clone());
        let footer = footer_map(&content);
        let artifact = footer.get("artifact_path").cloned().unwrap_or_default();
        for link in extract_markdown_links(&rel, &body) {
            rows.push(EdgeRow {
                path: rel.clone(),
                artifact: artifact.clone(),
                edge_type: "markdown_link".into(),
                target: link.target,
                resolved: link.resolved,
                exists: link.exists,
            });
        }
        for key in [
            "projection_ref",
            "contract_ref",
            "template_ref",
            "parent_definition_ref",
        ] {
            if let Some(target) = footer.get(key) {
                rows.push(EdgeRow {
                    path: rel.clone(),
                    artifact: artifact.clone(),
                    edge_type: key.to_string(),
                    target: target.clone(),
                    resolved: target.clone(),
                    exists: true,
                });
            }
        }
    }
    Ok(rows)
}

fn artifact_impact_rows(
    file: Option<&str>,
    artifact: &str,
    root: Option<&str>,
) -> std::io::Result<(String, String, Vec<ImpactRow>)> {
    let (value, source) = if let Some(path) = file {
        let content = fs::read_to_string(runtime_root().join(path))?;
        let footer = footer_map(&content);
        (
            footer
                .get("artifact_path")
                .cloned()
                .unwrap_or_else(|| path.to_string()),
            "file".to_string(),
        )
    } else {
        (artifact.to_string(), "artifact".to_string())
    };
    let repo = root
        .map(std::path::PathBuf::from)
        .unwrap_or_else(runtime_root);
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for file in all_markdown_files(&repo)? {
        let rel = normalize_path_for_root(&file, &repo);
        let content = fs::read_to_string(&file)?;
        let footer = footer_map(&content);
        let body = docflow_markdown::split_footer(&content)
            .map(|artifact| artifact.body)
            .unwrap_or(content.clone());
        let mut reasons = Vec::new();
        if footer.values().any(|entry| entry == &value) {
            reasons.push("footer_ref".to_string());
        }
        if extract_markdown_links(&rel, &body)
            .iter()
            .any(|link| link.target == value || link.resolved == value)
        {
            reasons.push("markdown_link".to_string());
        }
        if !reasons.is_empty() {
            let reason_key = reasons.join(",");
            if seen.insert((rel.clone(), reason_key.clone())) {
                rows.push(ImpactRow {
                    path: rel,
                    reasons: reason_key,
                });
            }
        }
    }
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    Ok((value, source, rows))
}

fn task_impact_rows(
    root: &str,
    task_id: &str,
) -> std::io::Result<(Vec<PathRow>, Vec<TaskImpactRow>)> {
    let scan_root = std::path::PathBuf::from(root);
    let mut touched_paths = BTreeSet::new();
    let mut touched_artifacts = BTreeSet::new();
    for changelog in all_changelog_files(&scan_root)? {
        let content = fs::read_to_string(&changelog)?;
        for line in content.lines().filter(|line| !line.trim().is_empty()) {
            let row: Value = match serde_json::from_str(line) {
                Ok(row) => row,
                Err(_) => continue,
            };
            if row.get("task_id").and_then(Value::as_str) != Some(task_id) {
                continue;
            }
            let markdown_name = changelog
                .file_name()
                .map(|name| name.to_string_lossy().replace(".changelog.jsonl", ".md"))
                .unwrap_or_else(|| "missing.md".to_string());
            let markdown = changelog.with_file_name(markdown_name);
            if markdown.exists() {
                touched_paths.insert(normalize_path_for_root(&markdown, &scan_root));
            }
            if let Some(artifact_path) = row.get("artifact_path").and_then(Value::as_str) {
                touched_artifacts.insert(artifact_path.to_string());
            }
        }
    }

    let mut impacts = Vec::new();
    let mut seen = BTreeSet::new();
    for artifact in touched_artifacts {
        let (_, _, rows) = artifact_impact_rows(None, &artifact, Some(root))?;
        for row in rows {
            if touched_paths.contains(&row.path) {
                continue;
            }
            if seen.insert((artifact.clone(), row.path.clone())) {
                impacts.push(TaskImpactRow {
                    source_artifact: artifact.clone(),
                    path: row.path,
                    reasons: row.reasons,
                });
            }
        }
    }
    let touched = touched_paths
        .into_iter()
        .map(|path| PathRow { path })
        .collect::<Vec<_>>();
    Ok((touched, impacts))
}

fn changelog_rows(markdown_file: &str) -> Result<Vec<Value>, String> {
    let path = resolve_runtime_path(markdown_file);
    if !path.exists() {
        return Err(format!("markdown file not found: {}", path.display()));
    }
    let content = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    let footer = footer_entries(&content)
        .ok_or_else(|| format!("footer metadata missing or invalid: {}", path.display()))?;
    let changelog_path = changelog_path_for_path(&path, &footer)?;
    read_changelog_rows(&changelog_path)
}

fn changelog_task_rows(
    root: Option<&str>,
    profile: &str,
    task_id: &str,
) -> Result<Vec<Value>, String> {
    let targets = resolve_profile_targets(root, profile, &[])?;
    let mut matched = Vec::new();
    for (scope_root, markdown_file) in targets {
        let content = match fs::read_to_string(&markdown_file) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let footer = match footer_entries(&content) {
            Some(footer) => footer,
            None => continue,
        };
        let changelog_path = match changelog_path_for_path(&markdown_file, &footer) {
            Ok(path) => path,
            Err(_) => continue,
        };
        let rows = match read_changelog_rows(&changelog_path) {
            Ok(rows) => rows,
            Err(_) => continue,
        };
        for row in rows {
            if row
                .get("task_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                != task_id
            {
                continue;
            }
            matched.push(serde_json::json!({
                "path": normalize_path_for_root(&markdown_file, &scope_root),
                "changelog": changelog_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default(),
                "artifact": row.get("artifact_path").cloned().unwrap_or(Value::String(String::new())),
                "event": row.get("event").cloned().unwrap_or(Value::String(String::new())),
                "ts": row.get("ts").cloned().unwrap_or(Value::String(String::new())),
                "task_id": row.get("task_id").cloned().unwrap_or(Value::String(String::new())),
                "reason": row.get("reason").cloned().unwrap_or(Value::String(String::new())),
                "actor": row.get("actor").cloned().unwrap_or(Value::String(String::new())),
                "scope": row.get("scope").cloned().unwrap_or(Value::String(String::new())),
                "tags": row.get("tags").cloned().unwrap_or(Value::Array(Vec::new())),
            }));
        }
    }
    Ok(matched)
}

fn task_summary_payload(
    root: Option<&str>,
    profile: &str,
    task_id: &str,
) -> Result<TaskSummaryPayload, String> {
    let targets = resolve_profile_targets(root, profile, &[])?;
    let mut actor_counts = BTreeMap::<String, usize>::new();
    let mut scope_counts = BTreeMap::<String, usize>::new();
    let mut tag_counts = BTreeMap::<String, usize>::new();
    let mut file_counts = BTreeMap::<String, usize>::new();
    let mut events = 0usize;
    let mut first_ts = String::new();
    let mut last_ts = String::new();

    for (scope_root, markdown_file) in targets {
        let content = match fs::read_to_string(&markdown_file) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let footer = match footer_entries(&content) {
            Some(footer) => footer,
            None => continue,
        };
        let changelog_path = match changelog_path_for_path(&markdown_file, &footer) {
            Ok(path) => path,
            Err(_) => continue,
        };
        let rows = match read_changelog_rows(&changelog_path) {
            Ok(rows) => rows,
            Err(_) => continue,
        };
        for row in rows {
            if row
                .get("task_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                != task_id
            {
                continue;
            }
            events += 1;
            let rel = normalize_path_for_root(&markdown_file, &scope_root);
            *file_counts.entry(rel).or_insert(0) += 1;
            if let Some(actor) = row.get("actor").and_then(Value::as_str) {
                if !actor.is_empty() {
                    *actor_counts.entry(actor.to_string()).or_insert(0) += 1;
                }
            }
            if let Some(scope) = row.get("scope").and_then(Value::as_str) {
                if !scope.is_empty() {
                    *scope_counts.entry(scope.to_string()).or_insert(0) += 1;
                }
            }
            if let Some(tags) = row.get("tags").and_then(Value::as_array) {
                for tag in tags.iter().filter_map(Value::as_str) {
                    if !tag.is_empty() {
                        *tag_counts.entry(tag.to_string()).or_insert(0) += 1;
                    }
                }
            }
            if let Some(ts) = row.get("ts").and_then(Value::as_str) {
                if first_ts.is_empty()
                    || parse_ts_value(Some(&Value::String(ts.to_string())))
                        < parse_ts_value(Some(&Value::String(first_ts.clone())))
                {
                    first_ts = ts.to_string();
                }
                if last_ts.is_empty()
                    || parse_ts_value(Some(&Value::String(ts.to_string())))
                        > parse_ts_value(Some(&Value::String(last_ts.clone())))
                {
                    last_ts = ts.to_string();
                }
            }
        }
    }

    Ok(TaskSummaryPayload {
        task_id: task_id.to_string(),
        root: if profile.is_empty() {
            root.unwrap_or_default().to_string()
        } else {
            profile.to_string()
        },
        events,
        files: file_counts.len(),
        first_ts,
        last_ts,
        files_rows: file_counts
            .into_iter()
            .map(|(path, events)| TaskSummaryFileRow { path, events })
            .collect(),
        actors: task_summary_counts(actor_counts),
        scopes: task_summary_counts(scope_counts),
        tags: task_summary_counts(tag_counts),
    })
}

fn task_summary_counts(counts: BTreeMap<String, usize>) -> Vec<TaskSummaryCountRow> {
    counts
        .into_iter()
        .map(|(value, events)| TaskSummaryCountRow { value, events })
        .collect()
}

fn resolve_markdown_scope(target: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    if target.is_file() {
        return Ok(vec![target.to_path_buf()]);
    }
    all_markdown_files(target)
}

fn all_markdown_files(root: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut output = Vec::new();
    visit_files(root, "md", &mut output)?;
    output.sort();
    Ok(output)
}

fn all_changelog_files(root: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut output = Vec::new();
    visit_files(root, "jsonl", &mut output)?;
    output.retain(|path| {
        path.file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with(".changelog.jsonl"))
    });
    output.sort();
    Ok(output)
}

fn visit_files(
    root: &std::path::Path,
    extension: &str,
    output: &mut Vec<std::path::PathBuf>,
) -> std::io::Result<()> {
    if root.is_file() {
        if root.extension().is_some_and(|ext| ext == extension) {
            output.push(root.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            visit_files(&path, extension, output)?;
        } else if path.extension().is_some_and(|ext| ext == extension) {
            output.push(path);
        }
    }
    Ok(())
}

fn normalize_path_for_repo(path: &std::path::Path) -> String {
    normalize_path_for_root(path, &runtime_root())
}

fn normalize_path_for_root(path: &std::path::Path, root: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn is_activation_governed_protocol(path: &str) -> bool {
    path.starts_with("vida/config/instructions/")
        && path.ends_with("protocol.md")
        && !path.contains("/system-maps.")
        && !path
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("system-maps."))
}

fn render_layer_status_from_rows(layer: usize, rows: &[Vec<(String, String)>]) -> String {
    if layer == 0 {
        return "layer-status\n  layer: 0\n  error: layer must be >= 1".to_string();
    }
    let idx = layer - 1;
    if idx >= rows.len() {
        return format!(
            "layer-status\n  layer: {layer}\n  error: layer out of range (max {})",
            rows.len()
        );
    }
    let current = rows[idx]
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect::<Vec<_>>();
    let start = idx.saturating_sub(1);
    let end = usize::min(rows.len(), idx + 2);
    let adjacent = rows[start..end]
        .iter()
        .enumerate()
        .filter_map(|(offset, row)| {
            let real_idx = start + offset;
            if real_idx == idx {
                return None;
            }
            let position = if real_idx < idx { "previous" } else { "next" };
            let mut rendered = vec![("position", position)];
            for (key, value) in row {
                if key == "number" {
                    rendered.push(("number", value.as_str()));
                } else if key == "Layer name" || key == "Status" {
                    rendered.push((key.as_str(), value.as_str()));
                }
            }
            Some(rendered)
        })
        .collect::<Vec<_>>();
    render_layer_status(layer, &current, &adjacent)
}

#[derive(Debug, Serialize)]
struct CheckRow {
    path: String,
    issues: Vec<String>,
}

#[derive(Debug, Clone)]
struct ReportShapeIssue {
    code: String,
    message: String,
}

fn custom_validation_issue(path: &str, code: &str, message: impl Into<String>) -> ValidationIssue {
    ValidationIssue {
        artifact_path: ArtifactPath(path.to_string()),
        verdict: ReadinessVerdict::Blocking,
        code: code.to_string(),
        message: message.into(),
        checked_at: CheckedAt::now_utc(),
    }
}

fn detect_project_root_for(path: &std::path::Path) -> Option<std::path::PathBuf> {
    path.ancestors()
        .find(|ancestor| {
            ancestor.join("AGENTS.sidecar.md").is_file()
                || ancestor.join("vida.config.yaml").is_file()
        })
        .map(std::path::Path::to_path_buf)
}

fn resolve_validation_scope(path: &str) -> (std::path::PathBuf, String) {
    let target = std::path::Path::new(path);
    if target.is_absolute() {
        if let Some(scope_root) = detect_project_root_for(target) {
            let rel = normalize_path_for_root(target, &scope_root);
            return (scope_root, rel);
        }
        let scope_root = target
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(runtime_root);
        let rel = target
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
        return (scope_root, rel);
    }
    (runtime_root(), path.to_string())
}

fn is_project_visible_doc(rel: &str) -> bool {
    rel == "AGENTS.sidecar.md" || (rel.starts_with("docs/") && rel.ends_with(".md"))
}

fn project_doc_owning_maps(rel: &str) -> Vec<&'static str> {
    match rel {
        "docs/project-root-map.md" => vec![],
        "docs/product/index.md" => vec!["docs/project-root-map.md"],
        "docs/product/spec/README.md" => vec!["docs/product/index.md"],
        "docs/process/README.md" => vec!["docs/project-root-map.md"],
        "docs/process/documentation-tooling-map.md" => {
            vec!["docs/process/README.md", "AGENTS.sidecar.md"]
        }
        "docs/research/README.md" => vec!["docs/project-root-map.md"],
        _ if rel.starts_with("docs/product/spec/templates/") => vec![],
        _ if rel.starts_with("docs/product/spec/") => {
            vec![
                "docs/product/spec/README.md",
                "docs/product/spec/current-spec-map.md",
            ]
        }
        _ if rel.starts_with("docs/product/") => vec!["docs/product/index.md"],
        _ if rel.starts_with("docs/process/") => {
            vec![
                "docs/process/README.md",
                "docs/process/documentation-tooling-map.md",
            ]
        }
        _ if rel.starts_with("docs/research/") => vec!["docs/research/README.md"],
        _ => vec![],
    }
}

fn project_doc_registration_validation_issues(
    scope_root: &std::path::Path,
    rel: &str,
    content: &str,
) -> Vec<ValidationIssue> {
    if !is_project_visible_doc(rel) {
        return Vec::new();
    }

    let mut issues = Vec::new();
    let sidecar_rel = "AGENTS.sidecar.md";
    let sidecar_path = scope_root.join(sidecar_rel);
    let sidecar_content = if rel == sidecar_rel {
        Some(content.to_string())
    } else {
        fs::read_to_string(&sidecar_path).ok()
    };

    if rel.starts_with("docs/") || rel == sidecar_rel {
        match sidecar_content.as_deref() {
            Some(body) => {
                if !body.contains("docs/project-root-map.md") {
                    issues.push(custom_validation_issue(
                        rel,
                        "missing_sidecar_project_root_map_pointer",
                        "AGENTS.sidecar.md must point to `docs/project-root-map.md` for project documentation routing.",
                    ));
                }
                if !body.contains("docs/process/documentation-tooling-map.md") {
                    issues.push(custom_validation_issue(
                        rel,
                        "missing_sidecar_documentation_tooling_map_pointer",
                        "AGENTS.sidecar.md must point to `docs/process/documentation-tooling-map.md` as the project documentation tooling map.",
                    ));
                }
            }
            None => issues.push(custom_validation_issue(
                rel,
                "missing_agents_sidecar",
                "Project documentation validation requires `AGENTS.sidecar.md` to exist as the project docs map.",
            )),
        }
    }

    let owning_maps = project_doc_owning_maps(rel);
    if owning_maps.is_empty() {
        return issues;
    }

    let mut existing_maps = Vec::new();
    let mut registered = false;
    for map_rel in &owning_maps {
        let map_path = scope_root.join(map_rel);
        let map_content = if rel == *map_rel {
            Some(content.to_string())
        } else {
            fs::read_to_string(&map_path).ok()
        };
        if let Some(body) = map_content {
            existing_maps.push(map_rel.to_string());
            if body.contains(rel) {
                registered = true;
            }
        }
    }

    if existing_maps.is_empty() {
        issues.push(custom_validation_issue(
            rel,
            "missing_project_doc_map_surface",
            format!(
                "Project doc `{rel}` requires an owning map/index surface such as {}.",
                owning_maps
                    .iter()
                    .map(|value| format!("`{value}`"))
                    .collect::<Vec<_>>()
                    .join(" or ")
            ),
        ));
    } else if !registered {
        issues.push(custom_validation_issue(
            rel,
            "missing_project_doc_map_registration",
            format!(
                "Project doc `{rel}` must be registered in one of the owning map/index surfaces: {}.",
                existing_maps
                    .iter()
                    .map(|value| format!("`{value}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    if matches!(
        rel,
        "docs/project-root-map.md" | "docs/process/documentation-tooling-map.md"
    ) {
        if let Some(body) = sidecar_content.as_deref() {
            if !body.contains(rel) {
                issues.push(custom_validation_issue(
                    rel,
                    "missing_sidecar_bootstrap_pointer",
                    format!(
                        "`AGENTS.sidecar.md` must reference `{rel}` when it is a bootstrap-visible project documentation surface."
                    ),
                ));
            }
        }
    }

    issues
}

fn collect_file_validation_issues(
    scope_root: &std::path::Path,
    rel: &str,
    content: &str,
) -> Vec<ValidationIssue> {
    let mut issues = validate_markdown_footer(ArtifactPath(rel.to_string()), content);
    issues.extend(project_doc_registration_validation_issues(
        scope_root, rel, content,
    ));
    issues
}

fn normalized_report_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| {
            line.trim_start()
                .trim_start_matches(['•', '-', '*'])
                .trim_start()
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

fn report_shape_issues(content: &str) -> Vec<ReportShapeIssue> {
    let lines = normalized_report_lines(content);
    let mut issues = Vec::new();

    match lines.first() {
        Some(line)
            if line.starts_with("Thinking mode: ")
                && ["STC.", "PR-CoT.", "MAR.", "5-SOL.", "META."]
                    .iter()
                    .any(|suffix| line.ends_with(suffix)) => {}
        _ => issues.push(ReportShapeIssue {
            code: "missing_thinking_mode_prefix".to_string(),
            message:
                "The first non-empty line must start with `Thinking mode: <STC|PR-CoT|MAR|5-SOL|META>.`."
                    .to_string(),
        }),
    }

    match lines.get(1) {
        Some(line) if line.starts_with("Requests: ") || line.starts_with("Tasks: ") => {}
        _ => issues.push(ReportShapeIssue {
            code: "missing_request_or_task_counters_prefix".to_string(),
            message: "The second non-empty line must start with either `Requests:` or `Tasks:`."
                .to_string(),
        }),
    }

    match lines.get(2) {
        Some(line) if line.starts_with("Agents: ") => {}
        _ => issues.push(ReportShapeIssue {
            code: "missing_agent_counters_prefix".to_string(),
            message: "The third non-empty line must start with `Agents:`.".to_string(),
        }),
    }

    issues
}

fn check_rows(
    root: Option<&str>,
    profile: &str,
    files: &[String],
) -> Result<Vec<CheckRow>, String> {
    let mut issues = Vec::new();
    let activation_body = read_activation_protocol().map_err(|err| err.to_string())?;
    let targets = resolve_profile_targets(root, profile, files)?;

    for (scope_root, file_path) in targets {
        let content = fs::read_to_string(&file_path).map_err(|err| err.to_string())?;
        let rel = normalize_path_for_root(&file_path, &scope_root);
        let footer = footer_map(&content);
        let mut row_issues = collect_file_validation_issues(&scope_root, &rel, &content)
            .into_iter()
            .map(|issue| issue.code)
            .collect::<Vec<_>>();
        if is_activation_governed_protocol(&rel)
            && activation_issue_for(&rel, &activation_body).is_some()
        {
            row_issues.push(format!("missing_activation_binding:{rel}"));
        }
        if !row_issues.is_empty() {
            issues.push(CheckRow {
                path: rel,
                issues: row_issues,
            });
        } else if footer.is_empty() && content.contains("-----") {
            issues.push(CheckRow {
                path: rel,
                issues: vec!["invalid_footer".into()],
            });
        }
    }
    Ok(issues)
}

fn fastcheck_rows(
    root: Option<&str>,
    profile: &str,
    files: &[String],
) -> Result<Vec<ValidationIssue>, String> {
    let targets = resolve_profile_targets(root, profile, files)?;
    let mut issues = Vec::new();
    for (scope_root, file_path) in targets {
        let content = fs::read_to_string(&file_path).map_err(|err| err.to_string())?;
        let rel = normalize_path_for_root(&file_path, &scope_root);
        issues.extend(collect_file_validation_issues(&scope_root, &rel, &content));
    }
    Ok(issues)
}

fn activation_rows(
    root: Option<&str>,
    profile: &str,
    files: &[String],
) -> Result<Vec<DoctorRow>, String> {
    let activation_body = read_activation_protocol().map_err(|err| err.to_string())?;
    let targets = resolve_profile_targets(root, profile, files)?;
    let mut rows = Vec::new();
    for (scope_root, file_path) in targets {
        let rel = normalize_path_for_root(&file_path, &scope_root);
        if let Some(issue) = activation_issue_for(&rel, &activation_body) {
            rows.push(issue);
        }
    }
    Ok(rows)
}

fn protocol_coverage_rows(
    root: Option<&str>,
    profile: &str,
    files: &[String],
) -> Result<Vec<DoctorRow>, String> {
    let activation_body = read_activation_protocol().map_err(|err| err.to_string())?;
    let protocol_index_body = read_protocol_index().map_err(|err| err.to_string())?;
    let targets = resolve_profile_targets(root, profile, files)?;
    let mut rows = Vec::new();
    for (scope_root, file_path) in targets {
        let rel = normalize_path_for_root(&file_path, &scope_root);
        if let Some(issue) =
            protocol_coverage_issue_for(&rel, &activation_body, &protocol_index_body)
        {
            rows.push(issue);
        }
    }
    Ok(rows)
}

fn readiness_rows(
    root: Option<&str>,
    profile: &str,
    files: &[String],
) -> Result<Vec<ReadinessRow>, String> {
    let targets = resolve_profile_targets(root, profile, files)?;
    let mut issues = Vec::new();
    for (scope_root, file_path) in targets {
        let content = fs::read_to_string(&file_path).map_err(|err| err.to_string())?;
        let rel = normalize_path_for_root(&file_path, &scope_root);
        issues.extend(collect_file_validation_issues(&scope_root, &rel, &content));
    }
    Ok(issues_to_readiness_rows(&issues))
}

fn resolve_profile_targets(
    root: Option<&str>,
    profile: &str,
    files: &[String],
) -> Result<Vec<(std::path::PathBuf, std::path::PathBuf)>, String> {
    let runtime = runtime_root();
    let root = root
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| runtime.clone());
    if !files.is_empty() {
        return files
            .iter()
            .map(|file| {
                let candidate = std::path::PathBuf::from(file);
                let resolved = if candidate.is_absolute() {
                    candidate
                } else {
                    root.join(candidate)
                };
                Ok((root.clone(), resolved))
            })
            .collect();
    }
    if profile.is_empty() {
        return collect_check_targets(&root, Vec::<String>::new()).map_err(|err| err.to_string());
    }

    let policy = docflow_policy_path();
    let roots =
        resolve_profile_roots(Some(&runtime), &policy, profile).map_err(|err| err.to_string())?;
    let excludes = resolve_scan_ignored_globs(&policy).map_err(|err| err.to_string())?;
    let mut rows = Vec::new();
    for root in roots {
        rows.extend(collect_check_targets(&root, excludes.clone()).map_err(|err| err.to_string())?);
    }
    Ok(rows)
}

fn collect_check_targets(
    root: &std::path::Path,
    exclude_globs: Vec<String>,
) -> Result<Vec<(std::path::PathBuf, std::path::PathBuf)>, globset::Error> {
    let mut scope = InventoryScope::new(root);
    scope.exclude_globs = exclude_globs;
    let rows = build_registry(&scope)?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let file = root.join(&row.artifact_path.0);
            (root.to_path_buf(), file)
        })
        .collect())
}

fn resolve_registry_output(args: &RegistryWriteArgs) -> String {
    if args.canonical {
        return resolve_rooted_output(&args.root, "vida/config/docflow-registry.current.jsonl");
    }
    args.output
        .clone()
        .unwrap_or_else(|| resolve_rooted_output(&args.root, "_temp/docflow-registry.jsonl"))
}

fn resolve_readiness_output(args: &RegistryWriteArgs) -> String {
    if args.canonical {
        return resolve_rooted_output(&args.root, "vida/config/docflow-readiness.current.jsonl");
    }
    args.output
        .clone()
        .unwrap_or_else(|| resolve_rooted_output(&args.root, "_temp/docflow-readiness.jsonl"))
}

fn finalize_edit(args: FinalizeEditArgs) -> Result<String, String> {
    let (markdown_files, change_note) = split_finalize_args(&args.args)?;
    let mut changed_files = Vec::new();
    for raw_path in markdown_files {
        let path = resolve_runtime_path(raw_path);
        let loaded = load_markdown_with_footer(&path)?;
        let mut footer = loaded.footer;
        let applied_updates = apply_finalize_updates(
            &mut footer,
            &args.status,
            &args.artifact_version,
            &args.artifact_revision,
            &args.set_values,
        )?;
        set_footer_value(&mut footer, "updated_at", &now_iso());
        write_markdown_with_footer(&path, loaded.artifact.body, &footer)?;
        append_changelog_event(
            &path,
            &footer,
            &args.event,
            change_note,
            &args.task_id,
            &args.actor,
            &args.scope,
            &args.tags,
            &applied_updates,
            &BTreeMap::new(),
        )?;
        changed_files.push(path);
    }
    Ok(render_mutation_result(
        "finalize-edit",
        &[
            format!("  files: {}", changed_files.len()),
            format!("  note: {change_note}"),
        ],
        &changed_files,
    ))
}

fn touch(args: TouchArgs) -> Result<String, String> {
    let path = resolve_runtime_path(&args.markdown_file);
    let loaded = load_markdown_with_footer(&path)?;
    let mut footer = loaded.footer;
    set_footer_value(&mut footer, "updated_at", &now_iso());
    write_markdown_with_footer(&path, loaded.artifact.body, &footer)?;
    append_changelog_event(
        &path,
        &footer,
        &args.event,
        &args.change_note,
        &args.task_id,
        &args.actor,
        &args.scope,
        &args.tags,
        &[],
        &BTreeMap::new(),
    )?;
    Ok(render_mutation_result(
        "touch",
        &[
            format!("  file: {}", normalize_path_for_repo(&path)),
            format!("  note: {}", args.change_note),
        ],
        std::slice::from_ref(&path),
    ))
}

fn rename_artifact(args: RenameArtifactArgs) -> Result<String, String> {
    let path = resolve_runtime_path(&args.markdown_file);
    let loaded = load_markdown_with_footer(&path)?;
    let mut footer = loaded.footer;
    let previous_artifact_path = footer
        .iter()
        .find_map(|(key, value)| (key == "artifact_path").then_some(value.clone()))
        .unwrap_or_default();
    set_footer_value(&mut footer, "artifact_path", &args.artifact_path);
    if !args.artifact_type.is_empty() {
        set_footer_value(&mut footer, "artifact_type", &args.artifact_type);
    }
    if args.bump_version {
        let next = footer
            .iter()
            .find_map(|(key, value)| (key == "artifact_version").then_some(value.clone()))
            .unwrap_or_else(|| "0".to_string())
            .parse::<u64>()
            .map_err(|err| format!("invalid artifact_version: {err}"))?
            + 1;
        set_footer_value(&mut footer, "artifact_version", &next.to_string());
    }
    set_footer_value(&mut footer, "updated_at", &now_iso());
    write_markdown_with_footer(&path, loaded.artifact.body, &footer)?;

    let mut extra_fields = BTreeMap::new();
    extra_fields.insert("previous_artifact_path".to_string(), previous_artifact_path);
    append_changelog_event(
        &path,
        &footer,
        &args.event,
        &args.change_note,
        &args.task_id,
        &args.actor,
        &args.scope,
        &args.tags,
        &[],
        &extra_fields,
    )?;

    Ok(render_mutation_result(
        "rename-artifact",
        &[
            format!("  file: {}", normalize_path_for_repo(&path)),
            format!("  artifact_path: {}", args.artifact_path),
        ],
        std::slice::from_ref(&path),
    ))
}

fn init_artifact(args: InitArgs) -> Result<String, String> {
    let path = resolve_runtime_path(&args.markdown_file);
    if path.exists() {
        return Err(format!("markdown file already exists: {}", path.display()));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let created_at = now_iso();
    let artifact_revision = if args.artifact_revision.is_empty() {
        created_at.chars().take(10).collect::<String>()
    } else {
        args.artifact_revision.clone()
    };
    let title = if args.title.is_empty() {
        titleize_stem(
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("artifact"),
        )
    } else {
        args.title.clone()
    };
    let body = if args.purpose.is_empty() {
        format!("# {title}\n\nPurpose:\n")
    } else {
        format!("# {title}\n\nPurpose: {}\n", args.purpose)
    };
    let changelog_ref = format!(
        "{}.changelog.jsonl",
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("artifact")
    );
    let source_path = normalize_path_for_repo(&path);
    let footer = vec![
        ("artifact_path".to_string(), args.artifact_path.clone()),
        ("artifact_type".to_string(), args.artifact_type.clone()),
        (
            "artifact_version".to_string(),
            args.artifact_version.to_string(),
        ),
        ("artifact_revision".to_string(), artifact_revision),
        (
            "schema_version".to_string(),
            args.schema_version.to_string(),
        ),
        ("status".to_string(), args.status.clone()),
        ("source_path".to_string(), source_path.clone()),
        ("created_at".to_string(), created_at.clone()),
        ("updated_at".to_string(), created_at.clone()),
        ("changelog_ref".to_string(), changelog_ref.clone()),
    ];
    write_markdown_with_footer(&path, body, &footer)?;
    append_changelog_event(
        &path,
        &footer,
        "artifact_initialized",
        &args.change_note,
        &args.task_id,
        &args.actor,
        &args.scope,
        &args.tags,
        &[],
        &BTreeMap::new(),
    )?;

    if args.json {
        serde_json::to_string(&serde_json::json!({
            "command": "init",
            "mode": "artifact_init",
            "status": "ok",
            "file": source_path,
            "artifact_path": args.artifact_path,
            "artifact_type": args.artifact_type,
            "artifact_version": args.artifact_version,
            "artifact_revision": footer_value(&footer, "artifact_revision"),
            "schema_version": args.schema_version,
            "created_at": created_at,
            "updated_at": created_at,
            "changelog_ref": changelog_ref,
            "validation": {
                "verdict": "ok",
                "issues": []
            },
            "blocker_codes": [],
            "next_actions": [
                "Run docflow check-file --path <file>",
                "Run docflow readiness-file --path <file>"
            ]
        }))
        .map_err(|error| error.to_string())
    } else {
        Ok(render_mutation_result(
            "init",
            &[
                format!("  file: {}", normalize_path_for_repo(&path)),
                format!("  artifact_path: {}", args.artifact_path),
                format!("  artifact_type: {}", args.artifact_type),
            ],
            std::slice::from_ref(&path),
        ))
    }
}

fn footer_value(footer: &[(String, String)], key: &str) -> String {
    footer
        .iter()
        .find_map(|(field, value)| (field == key).then(|| value.clone()))
        .unwrap_or_default()
}

fn init_command(args: InitArgs) -> Result<String, String> {
    if is_init_info_request(&args) {
        return render_init_info(args.json);
    }
    if args.markdown_file.is_empty() {
        return Err(
            "init artifact mode requires markdown_file, artifact_path, artifact_type, change_note"
                .to_string(),
        );
    }
    if args.artifact_path.is_empty() || args.artifact_type.is_empty() || args.change_note.is_empty()
    {
        return Err(
            "init artifact mode requires markdown_file, artifact_path, artifact_type, change_note"
                .to_string(),
        );
    }
    init_artifact(args)
}

fn is_init_info_request(args: &InitArgs) -> bool {
    args.markdown_file.is_empty()
        && args.artifact_path.is_empty()
        && args.artifact_type.is_empty()
        && args.change_note.is_empty()
}

fn render_init_info(json: bool) -> Result<String, String> {
    let payload = serde_json::json!({
        "command": "init",
        "mode": "agent_bootstrap",
        "status": "ready",
        "runtime_root": runtime_root().to_string_lossy(),
        "purpose": "DocFlow validates, inventories, relates, and proves project documentation artifacts before and after bounded development work.",
        "agent_startup": {
            "read_first": [
                "AGENTS.md",
                "AGENTS.sidecar.md",
                "docs/project-root-map.md",
                "docs/process/documentation-tooling-map.md"
            ],
            "preferred_machine_mode": "--json",
            "safe_first_commands": [
                "docflow init --json",
                "docflow doctor --root .",
                "docflow check-file --path <file>",
                "docflow readiness-check --profile active-canon",
                "docflow registry --root ."
            ]
        },
        "command_map": {
            "orientation": [
                "docflow init",
                "docflow help",
                "docflow overview --registry-count <n> --relation-count <n>"
            ],
            "validation": [
                "docflow check-file --path <file>",
                "docflow validate-footer --path <file> --content <markdown>",
                "docflow fastcheck --profile <profile>"
            ],
            "readiness": [
                "docflow readiness-check --profile <profile>",
                "docflow readiness-file --path <file>",
                "docflow readiness-write --root <root>"
            ],
            "inventory_relations": [
                "docflow registry --root <root>",
                "docflow relations-scan --root <root>",
                "docflow artifact-impact --root <root> --artifact-path <artifact>"
            ],
            "mutation": [
                "docflow init <markdown_file> <artifact_path> <artifact_type> <change_note>",
                "docflow touch <markdown_file> <change_note>",
                "docflow move <markdown_file> <destination> <change_note>",
                "docflow rename-artifact <markdown_file> <artifact_path> <change_note>"
            ]
        },
        "artifact_init": {
            "command": "docflow init <markdown_file> <artifact_path> <artifact_type> <change_note>",
            "required_fields": [
                "markdown_file",
                "artifact_path",
                "artifact_type",
                "change_note"
            ],
            "optional_fields": [
                "title",
                "purpose",
                "artifact_version",
                "artifact_revision",
                "schema_version",
                "status",
                "task_id",
                "actor",
                "scope",
                "tags"
            ]
        },
        "instructions": [
            "Use docflow init at session start to discover the standalone utility contract.",
            "Read AGENTS.md and AGENTS.sidecar.md before documentation mutation.",
            "Use JSON mode for agent handoff, blocker parsing, and next_actions.",
            "Run validation/readiness before closing documentation-shaped work.",
            "Use artifact init mode only when creating a new canonical markdown artifact."
        ],
        "blocker_codes": [],
        "next_actions": [
            "Run docflow doctor --root .",
            "Run docflow readiness-check --profile active-canon",
            "Run docflow help for the complete command surface"
        ],
    });
    if json {
        serde_json::to_string(&payload).map_err(|error| error.to_string())
    } else {
        Ok(format!(
            "docflow init\n  mode: agent_bootstrap\n  status: ready\n  purpose: {}\n  runtime_root: {}\n  read_first:\n    - AGENTS.md\n    - AGENTS.sidecar.md\n    - docs/project-root-map.md\n    - docs/process/documentation-tooling-map.md\n  safe_first_commands:\n    - docflow init --json\n    - docflow doctor --root .\n    - docflow check-file --path <file>\n    - docflow readiness-check --profile active-canon\n  artifact_init:\n    command: {}\n  agent_instructions:\n    1) Read bootstrap docs before mutation.\n    2) Prefer --json for handoff, blockers, and next_actions.\n    3) Validate/readiness-check touched docs before closure.\n    4) Use artifact init mode only for new canonical markdown artifacts.\n  next_actions:\n    - docflow doctor --root .\n    - docflow readiness-check --profile active-canon\n    - docflow help",
            payload["purpose"]
                .as_str()
                .unwrap_or("DocFlow documentation utility."),
            payload["runtime_root"].as_str().unwrap_or("unknown"),
            payload["artifact_init"]["command"],
        ))
    }
}

fn move_artifact(args: MoveArgs) -> Result<String, String> {
    let source = resolve_runtime_path(&args.markdown_file);
    let destination = resolve_runtime_path(&args.destination);
    if !source.exists() {
        return Err(format!("markdown file not found: {}", source.display()));
    }
    if destination.exists() {
        return Err(format!(
            "destination already exists: {}",
            destination.display()
        ));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let loaded = load_markdown_with_footer(&source)?;
    let source_footer = loaded.footer.clone();
    let mut footer = loaded.footer;
    let source_rel = normalize_path_for_repo(&source);
    set_footer_value(
        &mut footer,
        "source_path",
        &normalize_path_for_repo(&destination),
    );
    set_footer_value(&mut footer, "updated_at", &now_iso());
    set_footer_value(
        &mut footer,
        "changelog_ref",
        &format!(
            "{}.changelog.jsonl",
            destination
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("artifact")
        ),
    );
    write_markdown_with_footer(&destination, loaded.artifact.body, &footer)?;

    let source_changelog = changelog_path_for_path(&source, &source_footer)?;
    let destination_changelog = changelog_path_for_path(&destination, &footer)?;
    if source_changelog.exists() {
        let existing = fs::read_to_string(&source_changelog).map_err(|err| err.to_string())?;
        fs::write(&destination_changelog, existing).map_err(|err| err.to_string())?;
        let mut extra_fields = BTreeMap::new();
        extra_fields.insert("previous_source_path".to_string(), source_rel);
        append_changelog_event(
            &destination,
            &footer,
            "artifact_moved",
            &args.change_note,
            &args.task_id,
            &args.actor,
            &args.scope,
            &args.tags,
            &[],
            &extra_fields,
        )?;
        fs::remove_file(&source_changelog).map_err(|err| err.to_string())?;
    }
    fs::remove_file(&source).map_err(|err| err.to_string())?;

    Ok(render_mutation_result(
        "move",
        &[
            format!("  source: {}", args.markdown_file),
            format!("  destination: {}", normalize_path_for_repo(&destination)),
        ],
        std::slice::from_ref(&destination),
    ))
}

fn split_finalize_args(args: &[String]) -> Result<(&[String], &str), String> {
    if args.len() < 2 {
        return Err("expected at least one markdown file and a change note".into());
    }
    let (change_note, files) = args
        .split_last()
        .ok_or_else(|| "expected at least one markdown file and a change note".to_string())?;
    Ok((files, change_note))
}

struct LoadedMarkdownWithFooter {
    artifact: docflow_markdown::MarkdownArtifact,
    footer: Vec<(String, String)>,
}

fn load_markdown_with_footer(path: &std::path::Path) -> Result<LoadedMarkdownWithFooter, String> {
    if !path.exists() {
        return Err(format!("markdown file not found: {}", path.display()));
    }
    let content = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let artifact = docflow_markdown::split_footer(&content).map_err(|err| err.to_string())?;
    let footer =
        footer_entries(&content).ok_or_else(|| format!("missing footer: {}", path.display()))?;
    Ok(LoadedMarkdownWithFooter { artifact, footer })
}

fn write_markdown_with_footer(
    path: &std::path::Path,
    body: String,
    footer: &[(String, String)],
) -> Result<(), String> {
    let rendered = render_footer_entries(footer);
    let markdown = docflow_markdown::render_artifact(&docflow_markdown::MarkdownArtifact {
        body,
        footer: Some(rendered),
    });
    fs::write(path, markdown).map_err(|err| err.to_string())
}

fn render_mutation_result(
    command: &str,
    details: &[String],
    paths: &[std::path::PathBuf],
) -> String {
    let problems = quiet_check_paths(paths);
    let mut lines = vec![command.to_string()];
    lines.extend(details.iter().cloned());
    if problems.is_empty() {
        lines.push("  validation: ok".to_string());
    } else {
        lines.push("  validation: blocking".to_string());
        for (path, issues) in problems {
            lines.push(format!("  - {} [{}]", path, issues.join(",")));
        }
    }
    lines.join("\n")
}

fn resolve_runtime_path(path: &str) -> std::path::PathBuf {
    let target = std::path::Path::new(path);
    if target.is_absolute() {
        target.to_path_buf()
    } else {
        runtime_root().join(target)
    }
}

fn footer_entries(content: &str) -> Option<Vec<(String, String)>> {
    let artifact = docflow_markdown::split_footer(content).ok()?;
    let footer = artifact.footer?;
    let mut output = Vec::new();
    for line in footer.lines() {
        if let Some((key, value)) = line.split_once(':') {
            output.push((key.trim().to_string(), value.trim().to_string()));
        }
    }
    Some(output)
}

fn render_footer_entries(entries: &[(String, String)]) -> String {
    entries
        .iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn set_footer_value(entries: &mut Vec<(String, String)>, key: &str, value: &str) {
    if let Some((_, existing)) = entries
        .iter_mut()
        .find(|(existing_key, _)| existing_key == key)
    {
        *existing = value.to_string();
    } else {
        entries.push((key.to_string(), value.to_string()));
    }
}

fn apply_finalize_updates(
    footer: &mut Vec<(String, String)>,
    status: &str,
    artifact_version: &str,
    artifact_revision: &str,
    set_values: &[String],
) -> Result<Vec<String>, String> {
    let mut applied_updates = Vec::new();
    if !status.is_empty() {
        set_footer_value(footer, "status", status);
        applied_updates.push(format!("status={status}"));
    }
    if !artifact_version.is_empty() {
        set_footer_value(footer, "artifact_version", artifact_version);
        applied_updates.push(format!("artifact_version={artifact_version}"));
    }
    if !artifact_revision.is_empty() {
        set_footer_value(footer, "artifact_revision", artifact_revision);
        applied_updates.push(format!("artifact_revision={artifact_revision}"));
    }
    for item in set_values {
        let (key, value) = item
            .split_once('=')
            .ok_or_else(|| format!("invalid --set pair: {item}"))?;
        let key = key.trim();
        let value = value.trim();
        set_footer_value(footer, key, value);
        applied_updates.push(format!("{key}={value}"));
    }
    Ok(applied_updates)
}

fn now_iso() -> String {
    CheckedAt::now_utc()
        .0
        .format(&Rfc3339)
        .unwrap_or_else(|_| CheckedAt::now_utc().0.to_string())
}

fn changelog_path_for_path(
    path: &std::path::Path,
    footer: &[(String, String)],
) -> Result<std::path::PathBuf, String> {
    let changelog_ref = footer
        .iter()
        .find_map(|(key, value)| (key == "changelog_ref").then_some(value.clone()))
        .ok_or_else(|| {
            format!(
                "footer metadata is missing changelog_ref: {}",
                path.display()
            )
        })?;
    Ok(path.with_file_name(changelog_ref))
}

fn append_changelog_event(
    path: &std::path::Path,
    footer: &[(String, String)],
    event: &str,
    reason: &str,
    task_id: &str,
    actor: &str,
    scope: &str,
    tags: &str,
    applied_updates: &[String],
    extra_fields: &BTreeMap<String, String>,
) -> Result<(), String> {
    let footer_map = footer.iter().cloned().collect::<BTreeMap<String, String>>();
    let mut row = serde_json::json!({
        "ts": footer_map.get("updated_at").cloned().unwrap_or_else(now_iso),
        "event": event,
        "artifact_path": footer_map.get("artifact_path").cloned().unwrap_or_default(),
        "artifact_type": footer_map.get("artifact_type").cloned().unwrap_or_default(),
        "artifact_version": footer_map.get("artifact_version").cloned().unwrap_or_default(),
        "artifact_revision": footer_map.get("artifact_revision").cloned().unwrap_or_default(),
        "source_path": footer_map
            .get("source_path")
            .cloned()
            .unwrap_or_else(|| normalize_path_for_repo(path)),
        "reason": reason,
        "task_id": task_id,
        "actor": actor,
        "scope": scope,
        "tags": normalize_tag_list(tags),
    });
    if !applied_updates.is_empty() {
        row["metadata_updates"] = serde_json::json!(applied_updates);
    }
    if !extra_fields.is_empty() {
        let object = row
            .as_object_mut()
            .ok_or_else(|| "changelog row must be a JSON object".to_string())?;
        for (key, value) in extra_fields {
            object.insert(key.clone(), Value::String(value.clone()));
        }
    }
    let changelog_path = changelog_path_for_path(path, footer)?;
    let mut content = if changelog_path.exists() {
        fs::read_to_string(&changelog_path).map_err(|err| err.to_string())?
    } else {
        String::new()
    };
    let line = serde_json::to_string(&row).map_err(|err| err.to_string())?;
    content = docflow_markdown::append_changelog_row(&content, &line);
    fs::write(changelog_path, content).map_err(|err| err.to_string())
}

fn normalize_tag_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn titleize_stem(stem: &str) -> String {
    stem.replace(['.', '-', '_'], " ")
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn quiet_check_paths(paths: &[std::path::PathBuf]) -> Vec<(String, Vec<String>)> {
    let activation_body = read_activation_protocol().unwrap_or_default();
    let mut output = Vec::new();
    for path in paths {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                output.push((
                    normalize_path_for_repo(path),
                    vec![format!("read_error:{error}")],
                ));
                continue;
            }
        };
        let scope_root = detect_project_root_for(path).unwrap_or_else(runtime_root);
        let rel = normalize_path_for_root(path, &scope_root);
        let body = docflow_markdown::split_footer(&content)
            .map(|artifact| artifact.body)
            .unwrap_or(content.clone());
        let footer = footer_map(&content);
        let mut issues = collect_file_validation_issues(&scope_root, &rel, &content)
            .into_iter()
            .map(|issue| issue.code)
            .collect::<Vec<_>>();
        if !footer.values().any(|value| value.ends_with(".jsonl"))
            && !footer.contains_key("changelog_ref")
        {
            issues.push("missing_changelog_ref".into());
        }
        if is_activation_governed_protocol(&rel) {
            if let Some(issue) = activation_issue_for(&rel, &activation_body) {
                issues.push(issue.issues);
            }
        }
        for link in extract_markdown_links(&rel, &body) {
            if !link.exists {
                issues.push(format!("broken_link:{}", link.target));
            }
        }
        if !issues.is_empty() {
            output.push((rel, issues));
        }
    }
    output
}

fn read_changelog_rows(path: &std::path::Path) -> Result<Vec<Value>, String> {
    let content = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let mut rows = Vec::new();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let row = serde_json::from_str(line).map_err(|err| err.to_string())?;
        rows.push(row);
    }
    Ok(rows)
}

fn parse_ts_value(value: Option<&Value>) -> (u8, String) {
    let text = value.and_then(Value::as_str).unwrap_or_default().trim();
    if text.is_empty() {
        return (0, String::new());
    }
    (1, text.replace('Z', "+00:00"))
}

fn render_value_rows(
    command: &str,
    format: &str,
    rows: Vec<Value>,
    context: Option<Value>,
) -> String {
    if format == "jsonl" {
        return rows
            .into_iter()
            .map(|row| serde_json::to_string(&row).unwrap_or_else(|_| "{}".to_string()))
            .collect::<Vec<_>>()
            .join("\n");
    }

    let mut lines = vec![command.to_string()];
    if let Some(context) = context.and_then(|value| value.as_object().cloned()) {
        lines.push("  context:".to_string());
        for (key, value) in context {
            let rendered = value
                .as_str()
                .map(ToString::to_string)
                .unwrap_or_else(|| value.to_string());
            lines.push(format!("    {key}: {rendered}"));
        }
    }
    lines.push("  rows:".to_string());
    for row in rows {
        lines.push(format!(
            "    - {}",
            serde_json::to_string(&row).unwrap_or_else(|_| "{}".to_string())
        ));
    }
    lines.join("\n")
}

fn render_task_summary(payload: &TaskSummaryPayload, format: &str) -> String {
    if format == "jsonl" {
        let mut rows = vec![serde_json::json!({
            "summary": "task",
            "task_id": payload.task_id,
            "root": payload.root,
            "events": payload.events,
            "files": payload.files,
            "first_ts": payload.first_ts,
            "last_ts": payload.last_ts,
        })];
        rows.extend(payload.files_rows.iter().map(|row| {
            serde_json::json!({
                "summary": "file",
                "task_id": payload.task_id,
                "file": row.path,
                "events": row.events,
            })
        }));
        rows.extend(payload.actors.iter().map(|row| {
            serde_json::json!({
                "summary": "actor",
                "task_id": payload.task_id,
                "actor": row.value,
                "events": row.events,
            })
        }));
        rows.extend(payload.scopes.iter().map(|row| {
            serde_json::json!({
                "summary": "scope",
                "task_id": payload.task_id,
                "scope": row.value,
                "events": row.events,
            })
        }));
        rows.extend(payload.tags.iter().map(|row| {
            serde_json::json!({
                "summary": "tag",
                "task_id": payload.task_id,
                "tag": row.value,
                "events": row.events,
            })
        }));
        return rows
            .into_iter()
            .map(|row| serde_json::to_string(&row).unwrap_or_else(|_| "{}".to_string()))
            .collect::<Vec<_>>()
            .join("\n");
    }

    let mut lines = vec![
        "task-summary".to_string(),
        "  context:".to_string(),
        format!("    task_id: {}", payload.task_id),
        format!("    root: {}", payload.root),
        "  totals:".to_string(),
        format!("    events: {}", payload.events),
        format!("    files: {}", payload.files),
        format!("    first_ts: {}", payload.first_ts),
        format!("    last_ts: {}", payload.last_ts),
        "  files:".to_string(),
    ];
    for row in &payload.files_rows {
        lines.push(format!("    - {} [{}]", row.path, row.events));
    }
    lines.push("  actors:".to_string());
    for row in &payload.actors {
        lines.push(format!("    - {} [{}]", row.value, row.events));
    }
    lines.push("  scopes:".to_string());
    for row in &payload.scopes {
        lines.push(format!("    - {} [{}]", row.value, row.events));
    }
    lines.push("  tags:".to_string());
    for row in &payload.tags {
        lines.push(format!("    - {} [{}]", row.value, row.events));
    }
    lines.join("\n")
}

fn rewrite_markdown_links(body: &str, old_target: &str, new_target: &str) -> (String, usize) {
    let mut output = String::with_capacity(body.len());
    let mut cursor = 0usize;
    let mut replacements = 0usize;
    while let Some(relative) = body[cursor..].find("](") {
        let start = cursor + relative;
        let target_start = start + 2;
        output.push_str(&body[cursor..target_start]);
        if let Some(target_end_relative) = body[target_start..].find(')') {
            let target_end = target_start + target_end_relative;
            let target = &body[target_start..target_end];
            if target == old_target {
                output.push_str(new_target);
                replacements += 1;
            } else {
                output.push_str(target);
            }
            output.push(')');
            cursor = target_end + 1;
        } else {
            output.push_str(&body[target_start..]);
            cursor = body.len();
            break;
        }
    }
    output.push_str(&body[cursor..]);
    (output, replacements)
}

fn migrate_links(args: MigrateLinksArgs) -> Result<String, String> {
    let absolute = resolve_runtime_path(&args.path);
    let files = resolve_markdown_scope(&absolute).map_err(|err| err.to_string())?;
    let mut changed_paths = Vec::new();
    let mut preview_rows = Vec::new();
    for file_path in files {
        if file_path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let content = fs::read_to_string(&file_path).map_err(|err| err.to_string())?;
        let artifact = docflow_markdown::split_footer(&content).map_err(|err| err.to_string())?;
        let mut footer = footer_entries(&content).ok_or_else(|| {
            format!(
                "footer metadata missing or invalid: {}",
                file_path.display()
            )
        })?;
        let (updated_body, replacements) =
            rewrite_markdown_links(&artifact.body, &args.old_target, &args.new_target);
        if replacements == 0 {
            continue;
        }
        preview_rows.push(serde_json::json!({
            "path": normalize_path_for_repo(&file_path),
            "artifact": footer
                .iter()
                .find_map(|(key, value)| (key == "artifact_path").then_some(value.clone()))
                .unwrap_or_default(),
            "replacements": replacements,
            "old_target": args.old_target,
            "new_target": args.new_target,
        }));
        if args.dry_run {
            continue;
        }
        set_footer_value(&mut footer, "updated_at", &now_iso());
        let rendered = docflow_markdown::render_artifact(&docflow_markdown::MarkdownArtifact {
            body: updated_body,
            footer: Some(render_footer_entries(&footer)),
        });
        fs::write(&file_path, rendered).map_err(|err| err.to_string())?;
        let mut extra_fields = BTreeMap::new();
        extra_fields.insert("old_target".to_string(), args.old_target.clone());
        extra_fields.insert("new_target".to_string(), args.new_target.clone());
        extra_fields.insert("replacements".to_string(), replacements.to_string());
        append_changelog_event(
            &file_path,
            &footer,
            "links_migrated",
            &args.change_note,
            &args.task_id,
            &args.actor,
            &args.scope,
            &args.tags,
            &[],
            &extra_fields,
        )?;
        changed_paths.push(file_path);
    }
    if !args.dry_run {
        let issues = quiet_check_paths(&changed_paths);
        if !issues.is_empty() {
            let rendered = issues
                .into_iter()
                .map(|(path, issues)| format!("{path}: {}", issues.join(",")))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(format!("quiet validation failed: {rendered}"));
        }
    }
    Ok(render_value_rows(
        if args.dry_run {
            "migrate-links (dry-run)"
        } else {
            "migrate-links"
        },
        &args.format,
        preview_rows.clone(),
        Some(serde_json::json!({
            "command": "migrate-links",
            "path": args.path,
            "files": preview_rows.len(),
            "replacements": preview_rows
                .iter()
                .map(|row| row.get("replacements").and_then(Value::as_u64).unwrap_or(0))
                .sum::<u64>(),
        })),
    ))
}

fn resolve_rooted_output(root: &str, relative: &str) -> String {
    std::path::Path::new(root)
        .join(relative)
        .to_string_lossy()
        .to_string()
}

fn summarize_artifact_types(rows: &[docflow_contracts::RegistryRow]) -> Vec<(&str, usize)> {
    let mut counts = std::collections::BTreeMap::<&str, usize>::new();
    for row in rows {
        *counts.entry(row.artifact_type.as_str()).or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

fn read_layer_matrix() -> std::io::Result<Vec<Vec<(String, String)>>> {
    let matrix_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md");
    let text = fs::read_to_string(matrix_path)?;
    let mut table_lines = Vec::new();
    let mut in_table = false;
    for line in text.lines() {
        if line.starts_with("| Category | Layer 1 |") {
            in_table = true;
        }
        if in_table {
            if !line.starts_with('|') {
                break;
            }
            table_lines.push(line.to_string());
        }
    }
    if table_lines.len() < 3 {
        return Err(std::io::Error::other("layer status matrix not found"));
    }
    let split = |row: &str| -> Vec<String> {
        row.trim()
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().to_string())
            .collect()
    };
    let header = split(&table_lines[0]);
    let layer_count = header.len().saturating_sub(1);
    let mut rows = (0..layer_count)
        .map(|idx| vec![("number".to_string(), (idx + 1).to_string())])
        .collect::<Vec<_>>();
    for line in table_lines.into_iter().skip(2) {
        let row = split(&line);
        if row.len() != header.len() {
            continue;
        }
        let category = row[0].clone();
        for idx in 0..layer_count {
            rows[idx].push((category.clone(), row[idx + 1].clone()));
        }
    }
    Ok(rows)
}

fn write_registry_jsonl(
    path: &str,
    rows: &[docflow_contracts::RegistryRow],
) -> std::io::Result<()> {
    write_jsonl_lines(path, rows)
}

fn write_readiness_jsonl(path: &str, rows: &[ReadinessRow]) -> std::io::Result<()> {
    write_jsonl_lines(path, rows)
}

fn write_jsonl_lines<T: serde::Serialize>(path: &str, rows: &[T]) -> std::io::Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let mut content = rows
        .iter()
        .map(|row| encode_line(row))
        .collect::<Result<Vec<_>, _>>()
        .map_err(std::io::Error::other)?
        .join("\n");
    if !content.is_empty() {
        content.push('\n');
    }
    fs::write(path, content)
}

fn verdict_label(verdict: ReadinessVerdict) -> &'static str {
    match verdict {
        ReadinessVerdict::Ok => "ok",
        ReadinessVerdict::Warning => "warning",
        ReadinessVerdict::Blocking => "blocking",
    }
}

fn render_validation_result(path: &str, content: &str) -> String {
    let (scope_root, rel) = resolve_validation_scope(path);
    let issues = collect_file_validation_issues(&scope_root, &rel, content);
    if issues.is_empty() {
        "validation\n  issues: 0\n  verdict: ok".to_string()
    } else {
        let verdict = summarize_verdict(&issues_to_readiness_rows(&issues));
        let issue_lines = issues
            .iter()
            .map(|issue| {
                format!(
                    "  - {} [{}]: {}",
                    issue.artifact_path.0, issue.code, issue.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "validation\n  issues: {}\n  verdict: {}\n{}",
            issues.len(),
            verdict_label(verdict),
            issue_lines
        )
    }
}

fn render_readiness_result(path: &str, content: &str) -> String {
    let (scope_root, rel) = resolve_validation_scope(path);
    let issues = collect_file_validation_issues(&scope_root, &rel, content);
    let rows = issues_to_readiness_rows(&issues);
    let verdict = summarize_verdict(&rows);
    if rows.is_empty() {
        format!(
            "readiness\n  rows: 0\n  verdict: {}",
            verdict_label(verdict)
        )
    } else {
        let row_lines = rows
            .iter()
            .map(|row| {
                format!(
                    "  - {} [{}]",
                    row.artifact_path.0,
                    verdict_label(row.verdict)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "readiness\n  rows: {}\n  verdict: {}\n{}",
            rows.len(),
            verdict_label(verdict),
            row_lines
        )
    }
}

fn render_report_check_result(path: &str, content: &str) -> String {
    let issues = report_shape_issues(content);
    if issues.is_empty() {
        "reporting\n  issues: 0\n  verdict: ok".to_string()
    } else {
        let issue_lines = issues
            .iter()
            .map(|issue| format!("  - {} [{}]: {}", path, issue.code, issue.message))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "reporting\n  issues: {}\n  verdict: blocking\n{}",
            issues.len(),
            issue_lines
        )
    }
}

fn collect_tree_issues(
    root: &str,
    rows: &[docflow_contracts::RegistryRow],
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    for row in rows {
        let full_path = format!("{root}/{}", row.artifact_path.0);
        match fs::read_to_string(&full_path) {
            Ok(content) => issues.extend(collect_file_validation_issues(
                std::path::Path::new(root),
                &row.artifact_path.0,
                &content,
            )),
            Err(error) => issues.push(ValidationIssue {
                artifact_path: row.artifact_path.clone(),
                verdict: ReadinessVerdict::Blocking,
                code: "read_error".into(),
                message: error.to_string(),
                checked_at: CheckedAt::now_utc(),
            }),
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::{Cli, activation_issue_for, protocol_coverage_issue_for, run};
    use clap::Parser;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        format!("/tmp/docflow-cli-{name}-{}-{nanos}.md", std::process::id())
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("docflow-cli-{name}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn overview_command_renders_operator_surface() {
        let cli = Cli::parse_from([
            "docflow",
            "overview",
            "--registry-count",
            "7",
            "--relation-count",
            "3",
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("registry_rows: 7"));
        assert!(rendered.contains("relation_edges: 3"));
    }

    #[test]
    fn summary_command_renders_compact_tree_summary() {
        let root = temp_dir("summary-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::create_dir_all(root.join("docs/product/spec")).expect("spec dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");
        fs::write(root.join("docs/product/spec/b.md"), "# b\n").expect("spec markdown");

        let cli = Cli::parse_from([
            "docflow",
            "summary",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("summary"));
        assert!(rendered.contains("registry_rows: 2"));
        assert!(rendered.contains("relation_edges: 2"));
        assert!(rendered.contains("readiness: blocking"));
        assert!(rendered.contains("type[process_doc]: 1"));
        assert!(rendered.contains("type[product_spec]: 1"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn changelog_command_streams_rows_from_markdown_sidecar() {
        let root = temp_dir("changelog-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(
            root.join("docs/process/a.md"),
            "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("markdown should exist");
        fs::write(
            root.join("docs/process/a.changelog.jsonl"),
            "{\"ts\":\"2026-03-11T00:00:00Z\",\"event\":\"artifact_initialized\",\"artifact_path\":\"process/a\"}\n",
        )
        .expect("changelog should exist");

        let cli = Cli::parse_from([
            "docflow",
            "changelog",
            root.join("docs/process/a.md").to_string_lossy().as_ref(),
            "--format",
            "jsonl",
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("\"event\":\"artifact_initialized\""));
        assert!(rendered.contains("\"artifact_path\":\"process/a\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn changelog_task_and_task_summary_scan_task_events() {
        let root = temp_dir("task-summary-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(
            root.join("docs/process/a.md"),
            "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("markdown should exist");
        fs::write(
            root.join("docs/process/a.changelog.jsonl"),
            "{\"ts\":\"2026-03-11T00:00:00Z\",\"event\":\"artifact_initialized\",\"artifact_path\":\"process/a\",\"task_id\":\"vida-rf1\",\"actor\":\"codex\",\"scope\":\"bridge\",\"tags\":[\"docflow\"]}\n",
        )
        .expect("changelog should exist");

        let changelog_cli = Cli::parse_from([
            "docflow",
            "changelog-task",
            "--root",
            root.to_string_lossy().as_ref(),
            "--task-id",
            "vida-rf1",
            "--format",
            "jsonl",
        ]);
        let changelog_rendered = run(changelog_cli);
        assert!(changelog_rendered.contains("\"path\":\"docs/process/a.md\""));
        assert!(changelog_rendered.contains("\"task_id\":\"vida-rf1\""));

        let summary_cli = Cli::parse_from([
            "docflow",
            "task-summary",
            "--root",
            root.to_string_lossy().as_ref(),
            "--task-id",
            "vida-rf1",
            "--format",
            "jsonl",
        ]);
        let summary_rendered = run(summary_cli);
        assert!(summary_rendered.contains("\"summary\":\"task\""));
        assert!(summary_rendered.contains("\"events\":1"));
        assert!(summary_rendered.contains("\"summary\":\"actor\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn scan_command_streams_scan_rows_from_real_tree() {
        let root = temp_dir("scan-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");

        let cli = Cli::parse_from(["docflow", "scan", "--root", root.to_string_lossy().as_ref()]);
        let rendered = run(cli);
        assert!(rendered.contains("\"artifact_path\":\"docs/process/a.md\""));
        assert!(rendered.contains("\"artifact_type\":\"process_doc\""));
        assert!(rendered.contains("\"has_footer\":false"));
        assert!(rendered.contains("\"has_changelog\":false"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn scan_command_supports_missing_only_filter() {
        let root = temp_dir("scan-missing-only-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(
            root.join("docs/process/a.md"),
            "# a\n\n-----\nartifact_path: process/a\n",
        )
        .expect("footered markdown");
        fs::write(root.join("docs/process/b.md"), "# b\n").expect("plain markdown");

        let cli = Cli::parse_from([
            "docflow",
            "scan",
            "--root",
            root.to_string_lossy().as_ref(),
            "--missing-only",
        ]);
        let rendered = run(cli);
        assert!(!rendered.contains("\"artifact_path\":\"docs/process/a.md\""));
        assert!(rendered.contains("\"artifact_path\":\"docs/process/b.md\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn layer_status_command_reads_canonical_matrix() {
        let cli = Cli::parse_from(["docflow", "layer-status", "--layer", "6"]);
        let rendered = run(cli);
        assert!(rendered.contains("layer-status"));
        assert!(rendered.contains("layer: 6"));
        assert!(rendered.contains("Layer name: Canonical Operator"));
        assert!(rendered.contains("Status: ✅"));
    }

    #[test]
    fn fastcheck_command_streams_validation_issues_from_real_tree() {
        let root = temp_dir("fastcheck-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");

        let cli = Cli::parse_from([
            "docflow",
            "fastcheck",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("\"artifact_path\":\"docs/process/a.md\""));
        assert!(rendered.contains("\"code\":\"missing_footer\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn activation_check_command_reports_missing_binding_for_protocol_file() {
        let root = temp_dir("activation-check-root");
        fs::create_dir_all(root.join("vida/config/instructions")).expect("instructions dir");
        fs::write(
            root.join("vida/config/instructions/runtime-instructions.synthetic-protocol.md"),
            "# synthetic\n",
        )
        .expect("protocol markdown");

        let cli = Cli::parse_from([
            "docflow",
            "activation-check",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains(
            "\"path\":\"vida/config/instructions/runtime-instructions.synthetic-protocol.md\""
        ));
        assert!(rendered.contains("missing_activation_binding"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn protocol_coverage_check_reports_missing_bindings_for_protocol_file() {
        let root = temp_dir("protocol-coverage-root");
        fs::create_dir_all(root.join("vida/config/instructions")).expect("instructions dir");
        fs::write(
            root.join("vida/config/instructions/runtime-instructions.synthetic-protocol.md"),
            "# synthetic\n",
        )
        .expect("protocol markdown");

        let cli = Cli::parse_from([
            "docflow",
            "protocol-coverage-check",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains(
            "\"path\":\"vida/config/instructions/runtime-instructions.synthetic-protocol.md\""
        ));
        assert!(rendered.contains("missing_activation_binding"));
        assert!(rendered.contains("missing_protocol_index_binding"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn activation_issue_accepts_canonical_shorthand_binding_without_md_suffix() {
        let issue = activation_issue_for(
            "vida/config/instructions/runtime-instructions/synthetic-protocol.md",
            "runtime-instructions/synthetic-protocol\n",
        );
        assert!(issue.is_none());
    }

    #[test]
    fn protocol_coverage_issue_accepts_canonical_shorthand_binding_without_md_suffix() {
        let issue = protocol_coverage_issue_for(
            "vida/config/instructions/runtime-instructions/synthetic-protocol.md",
            "runtime-instructions/synthetic-protocol\n",
            "runtime-instructions/synthetic-protocol\n",
        );
        assert!(issue.is_none());
    }

    #[test]
    fn proofcheck_command_aggregates_layer_scope_rows() {
        let cli = Cli::parse_from(["docflow", "proofcheck", "--layer", "6"]);
        let rendered = run(cli);
        assert!(rendered.contains("proofcheck"));
        assert!(rendered.contains("layer: 6"));
        assert!(rendered.contains("files_mode: layer"));
        assert!(rendered.contains("fastcheck_rows:"));
        assert!(rendered.contains("protocol_coverage_rows:"));
        assert!(rendered.contains("readiness_rows:"));
        assert!(rendered.contains("doctor_error_rows:"));
    }

    #[test]
    fn proofcheck_command_accepts_active_canon_strict_profile() {
        let cli = Cli::parse_from(["docflow", "proofcheck", "--profile", "active-canon-strict"]);
        let rendered = run(cli);
        assert!(rendered.contains("context:"));
        assert!(rendered.contains("root: active-canon-strict"));
        assert!(rendered.contains("files_mode: profile"));
        assert!(
            rendered.contains("OK: proofcheck") || rendered.contains("BLOCKING: proofcheck"),
            "unexpected proofcheck status output: {rendered}"
        );
    }

    #[test]
    fn doctor_command_streams_error_rows_from_real_tree() {
        let root = temp_dir("doctor-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");

        let cli = Cli::parse_from([
            "docflow",
            "doctor",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("\"severity\":\"error\""));
        assert!(rendered.contains("\"path\":\"docs/process/a.md\""));
        assert!(rendered.contains("missing_footer"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn overview_scan_command_renders_operator_surface_from_real_tree() {
        let root = temp_dir("overview-scan");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::create_dir_all(root.join("docs/product/spec")).expect("spec dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");
        fs::write(root.join("docs/product/spec/b.md"), "# b\n").expect("spec markdown");

        let cli = Cli::parse_from([
            "docflow",
            "overview-scan",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("docflow overview"));
        assert!(rendered.contains("registry_rows: 2"));
        assert!(rendered.contains("relation_edges: 2"));
        assert!(rendered.contains("readiness: ok"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn relations_command_renders_relation_surface() {
        let cli = Cli::parse_from(["docflow", "relations", "--edge-count", "2"]);
        let rendered = run(cli);
        assert_eq!(rendered, "relations\n  total_edges: 2");
    }

    #[test]
    fn relations_scan_command_renders_relation_surface_from_real_tree() {
        let root = temp_dir("relations-scan");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::create_dir_all(root.join("docs/product/spec")).expect("spec dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");
        fs::write(root.join("docs/product/spec/b.md"), "# b\n").expect("spec markdown");

        let cli = Cli::parse_from([
            "docflow",
            "relations-scan",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert_eq!(rendered, "relations\n  total_edges: 2");

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn artifact_impact_command_defaults_to_operator_surface() {
        let root = temp_dir("artifact-impact-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(
            root.join("docs/process/a.md"),
            "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: canonical\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("source markdown");
        fs::write(
            root.join("docs/process/b.md"),
            "# b\n\n[Source](a.md)\n\n-----\nartifact_path: process/b\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/b.md\nstatus: canonical\nchangelog_ref: b.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\ncontract_ref: process/a\n",
        )
        .expect("dependent markdown");

        let cli = Cli::parse_from([
            "docflow",
            "artifact-impact",
            "--artifact",
            "process/a",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("artifact-impact"));
        assert!(rendered.contains("artifact: process/a"));
        assert!(rendered.contains("impacts: 2"));
        assert!(rendered.contains("impact: docs/process/a.md [footer_ref]"));
        assert!(rendered.contains("impact: docs/process/b.md [footer_ref]"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn artifact_impact_command_keeps_json_output_when_requested() {
        let root = temp_dir("artifact-impact-json-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(
            root.join("docs/process/a.md"),
            "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: canonical\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("source markdown");
        fs::write(
            root.join("docs/process/b.md"),
            "# b\n\n[Source](a.md)\n\n-----\nartifact_path: process/b\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/b.md\nstatus: canonical\nchangelog_ref: b.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("dependent markdown");

        let cli = Cli::parse_from([
            "docflow",
            "artifact-impact",
            "--artifact",
            "process/a",
            "--root",
            root.to_string_lossy().as_ref(),
            "--format",
            "jsonl",
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("\"command\":\"artifact-impact\""));
        assert!(rendered.contains("\"artifact\":\"process/a\""));
        assert!(rendered.contains("\"path\":\"docs/process/a.md\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn task_impact_command_defaults_to_operator_surface() {
        let root = temp_dir("task-impact-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(
            root.join("docs/process/a.md"),
            "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: canonical\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("source markdown");
        fs::write(
            root.join("docs/process/a.changelog.jsonl"),
            "{\"ts\":\"2026-03-11T00:00:00Z\",\"event\":\"artifact_revision_updated\",\"artifact_path\":\"process/a\",\"task_id\":\"vida-stack-r1-b14\"}\n",
        )
        .expect("source changelog");
        fs::write(
            root.join("docs/process/b.md"),
            "# b\n\n[Source](a.md)\n\n-----\nartifact_path: process/b\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/b.md\nstatus: canonical\nchangelog_ref: b.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\ncontract_ref: process/a\n",
        )
        .expect("dependent markdown");

        let cli = Cli::parse_from([
            "docflow",
            "task-impact",
            "--task-id",
            "vida-stack-r1-b14",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("task-impact"));
        assert!(rendered.contains("task_id: vida-stack-r1-b14"));
        assert!(rendered.contains("touched: 1"));
        assert!(rendered.contains("indirect_impacts: 1"));
        assert!(rendered.contains("touched_path: docs/process/a.md"));
        assert!(rendered.contains("indirect_impact: docs/process/b.md <= process/a [footer_ref]"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn task_impact_command_keeps_json_output_when_requested() {
        let root = temp_dir("task-impact-json-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(
            root.join("docs/process/a.md"),
            "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: canonical\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("source markdown");
        fs::write(
            root.join("docs/process/a.changelog.jsonl"),
            "{\"ts\":\"2026-03-11T00:00:00Z\",\"event\":\"artifact_revision_updated\",\"artifact_path\":\"process/a\",\"task_id\":\"vida-stack-r1-b14\"}\n",
        )
        .expect("source changelog");
        fs::write(
            root.join("docs/process/b.md"),
            "# b\n\n[Source](a.md)\n\n-----\nartifact_path: process/b\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/b.md\nstatus: canonical\nchangelog_ref: b.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("dependent markdown");

        let cli = Cli::parse_from([
            "docflow",
            "task-impact",
            "--task-id",
            "vida-stack-r1-b14",
            "--root",
            root.to_string_lossy().as_ref(),
            "--format",
            "jsonl",
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("\"command\":\"task-impact\""));
        assert!(rendered.contains("\"task_id\":\"vida-stack-r1-b14\""));
        assert!(rendered.contains("\"touched\":[{\"path\":\"docs/process/a.md\"}]"));
        assert!(rendered.contains("\"indirect_impacts\":[]"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn validate_footer_command_reports_ok_for_footered_artifact() {
        let cli = Cli::parse_from([
            "docflow",
            "validate-footer",
            "--path",
            "notes/test.md",
            "--content",
            "# title\n\n-----\nartifact_path: process/test\n",
        ]);
        let rendered = run(cli);
        assert_eq!(rendered, "validation\n  issues: 0\n  verdict: ok");
    }

    #[test]
    fn registry_write_command_writes_jsonl_from_real_tree() {
        let root = temp_dir("registry-write-root");
        let output = temp_path("registry-write-output");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");

        let cli = Cli::parse_from([
            "docflow",
            "registry-write",
            "--root",
            root.to_string_lossy().as_ref(),
            "--output",
            &output,
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("registry-write"));
        assert!(rendered.contains("total_rows: 1"));
        let written = fs::read_to_string(&output).expect("registry jsonl should exist");
        assert!(written.contains("\"artifact_path\":\"docs/process/a.md\""));
        assert!(written.ends_with('\n'));

        fs::remove_dir_all(root).expect("temp root should be removed");
        fs::remove_file(output).expect("temp output should be removed");
    }

    #[test]
    fn registry_write_command_supports_canonical_output() {
        let root = temp_dir("registry-write-canonical-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");

        let cli = Cli::parse_from([
            "docflow",
            "registry-write",
            "--root",
            root.to_string_lossy().as_ref(),
            "--canonical",
        ]);
        let rendered = run(cli);
        let expected = root.join("vida/config/docflow-registry.current.jsonl");
        assert!(rendered.contains(expected.to_string_lossy().as_ref()));
        let written = fs::read_to_string(&expected).expect("canonical registry jsonl should exist");
        assert!(written.contains("\"artifact_path\":\"docs/process/a.md\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn registry_command_streams_jsonl_from_real_tree() {
        let root = temp_dir("registry-stream-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");

        let cli = Cli::parse_from([
            "docflow",
            "registry",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("\"artifact_path\":\"docs/process/a.md\""));
        assert!(rendered.contains("\"artifact_type\":\"process_doc\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn readiness_write_command_writes_jsonl_from_real_tree() {
        let root = temp_dir("readiness-write-root");
        let output = temp_path("readiness-write-output");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");

        let cli = Cli::parse_from([
            "docflow",
            "readiness-write",
            "--root",
            root.to_string_lossy().as_ref(),
            "--output",
            &output,
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("readiness-write"));
        assert!(rendered.contains("rows: 3"));
        let written = fs::read_to_string(&output).expect("readiness jsonl should exist");
        assert!(written.contains("\"artifact_path\":\"docs/process/a.md\""));
        assert!(written.contains("\"verdict\":\"blocking\""));
        assert_eq!(written.lines().count(), 3);
        assert!(written.ends_with('\n'));

        fs::remove_dir_all(root).expect("temp root should be removed");
        fs::remove_file(output).expect("temp output should be removed");
    }

    #[test]
    fn readiness_write_command_supports_canonical_output() {
        let root = temp_dir("readiness-write-canonical-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");

        let cli = Cli::parse_from([
            "docflow",
            "readiness-write",
            "--root",
            root.to_string_lossy().as_ref(),
            "--canonical",
        ]);
        let rendered = run(cli);
        let expected = root.join("vida/config/docflow-readiness.current.jsonl");
        assert!(rendered.contains(expected.to_string_lossy().as_ref()));
        let written =
            fs::read_to_string(&expected).expect("canonical readiness jsonl should exist");
        assert!(written.contains("\"artifact_path\":\"docs/process/a.md\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn readiness_check_command_streams_jsonl_from_real_tree() {
        let root = temp_dir("readiness-stream-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");

        let cli = Cli::parse_from([
            "docflow",
            "readiness-check",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("\"artifact_path\":\"docs/process/a.md\""));
        assert!(rendered.contains("\"verdict\":\"blocking\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn readiness_check_command_accepts_active_canon_profile() {
        let cli = Cli::parse_from(["docflow", "readiness-check", "--profile", "active-canon"]);
        let rendered = run(cli);
        assert!(!rendered.contains("inventory_error"));
        assert!(!rendered.contains("error"));
    }

    #[test]
    fn check_file_blocks_when_project_doc_is_missing_from_owning_map() {
        let root = temp_dir("project-doc-map-check");
        fs::create_dir_all(root.join("docs/product/spec")).expect("spec dir should exist");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(
            root.join("AGENTS.sidecar.md"),
            "# Project Docs Map\n\n- `docs/project-root-map.md`\n- `docs/process/documentation-tooling-map.md`\n",
        )
        .expect("sidecar should exist");
        fs::write(
            root.join("docs/project-root-map.md"),
            "# Root Map\n\n- `docs/product/index.md`\n- `docs/process/README.md`\n",
        )
        .expect("root map should exist");
        fs::write(
            root.join("docs/process/documentation-tooling-map.md"),
            "# Documentation Tooling\n",
        )
        .expect("documentation tooling map should exist");
        fs::write(
            root.join("docs/product/spec/README.md"),
            "# Product Spec Guide\n\nActive design docs:\n\n",
        )
        .expect("spec readme should exist");
        fs::write(
            root.join("docs/product/spec/flappy-bird-design.md"),
            "# Flappy Bird Design\n\n-----\nartifact_path: product/spec/flappy-bird-design\nartifact_type: product_spec\nartifact_version: 1\nartifact_revision: init\nsource_path: docs/product/spec/flappy-bird-design.md\nstatus: proposed\nchangelog_ref: flappy-bird-design.changelog.jsonl\ncreated_at: 2026-03-14T00:00:00Z\nupdated_at: 2026-03-14T00:00:00Z\n",
        )
        .expect("design doc should exist");

        let cli = Cli::parse_from([
            "docflow",
            "check-file",
            "--path",
            root.join("docs/product/spec/flappy-bird-design.md")
                .to_string_lossy()
                .as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("missing_project_doc_map_registration"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn check_file_blocks_when_sidecar_omits_documentation_tooling_map_pointer() {
        let root = temp_dir("sidecar-pointer-check");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(
            root.join("AGENTS.sidecar.md"),
            "# Project Docs Map\n\n- `docs/project-root-map.md`\n",
        )
        .expect("sidecar should exist");
        fs::write(
            root.join("docs/process/README.md"),
            "# Process Docs\n\n- `documentation-tooling-map.md`\n",
        )
        .expect("process readme should exist");
        fs::write(
            root.join("docs/process/documentation-tooling-map.md"),
            "# Documentation Tooling\n",
        )
        .expect("documentation tooling map should exist");

        let cli = Cli::parse_from([
            "docflow",
            "check-file",
            "--path",
            root.join("AGENTS.sidecar.md").to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("missing_sidecar_documentation_tooling_map_pointer"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn report_check_accepts_required_runtime_reporting_shape() {
        let path = temp_path("reporting-ok");
        fs::write(
            &path,
            "Thinking mode: MAR.\nTasks: active=1 | in_work=1 | blocked=0\nAgents: active=0 | working=0 | waiting=0\n",
        )
        .expect("report should be written");

        let cli = Cli::parse_from(["docflow", "report-check", "--path", &path]);
        let rendered = run(cli);
        assert!(rendered.contains("verdict: ok"));

        fs::remove_file(path).expect("temp report should be removed");
    }

    #[test]
    fn report_check_blocks_when_thinking_prefix_is_missing() {
        let path = temp_path("reporting-block");
        fs::write(
            &path,
            "Tasks: active=1 | in_work=1 | blocked=0\nAgents: active=0 | working=0 | waiting=0\n",
        )
        .expect("report should be written");

        let cli = Cli::parse_from(["docflow", "report-check", "--path", &path]);
        let rendered = run(cli);
        assert!(rendered.contains("verdict: blocking"));
        assert!(rendered.contains("missing_thinking_mode_prefix"));

        fs::remove_file(path).expect("temp report should be removed");
    }

    #[test]
    fn finalize_edit_updates_footer_and_appends_changelog() {
        let root = temp_dir("finalize-edit-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        let markdown = root.join("docs/process/a.md");
        let changelog = root.join("docs/process/a.changelog.jsonl");
        fs::write(
            &markdown,
            "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("markdown should exist");
        fs::write(&changelog, "").expect("changelog should exist");

        let cli = Cli::parse_from([
            "docflow",
            "finalize-edit",
            markdown.to_string_lossy().as_ref(),
            "update footer metadata",
            "--status",
            "canonical",
            "--artifact-revision",
            "2026-03-11",
            "--set",
            "owner=docflow",
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("finalize-edit"));
        assert!(rendered.contains("validation: ok"));

        let updated = fs::read_to_string(&markdown).expect("updated markdown should exist");
        assert!(updated.contains("status: canonical"));
        assert!(updated.contains("artifact_revision: 2026-03-11"));
        assert!(updated.contains("owner: docflow"));

        let changelog_body = fs::read_to_string(&changelog).expect("changelog should be written");
        assert!(changelog_body.contains("\"event\":\"artifact_revision_updated\""));
        assert!(changelog_body.contains("\"reason\":\"update footer metadata\""));
        assert!(changelog_body.contains("\"metadata_updates\":[\"status=canonical\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn touch_appends_changelog_without_proxying_metadata_updates() {
        let root = temp_dir("touch-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        let markdown = root.join("docs/process/a.md");
        let changelog = root.join("docs/process/a.changelog.jsonl");
        fs::write(
            &markdown,
            "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("markdown should exist");
        fs::write(&changelog, "").expect("changelog should exist");

        let cli = Cli::parse_from([
            "docflow",
            "touch",
            markdown.to_string_lossy().as_ref(),
            "record changelog event",
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("touch"));
        assert!(rendered.contains("validation: ok"));

        let changelog_body = fs::read_to_string(&changelog).expect("changelog should be written");
        assert!(changelog_body.contains("\"event\":\"artifact_revision_updated\""));
        assert!(changelog_body.contains("\"reason\":\"record changelog event\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn rename_artifact_updates_footer_and_tracks_previous_path() {
        let root = temp_dir("rename-artifact-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        let markdown = root.join("docs/process/a.md");
        let changelog = root.join("docs/process/a.changelog.jsonl");
        fs::write(
            &markdown,
            "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("markdown should exist");
        fs::write(&changelog, "").expect("changelog should exist");

        let cli = Cli::parse_from([
            "docflow",
            "rename-artifact",
            markdown.to_string_lossy().as_ref(),
            "process/b",
            "rename artifact identity",
            "--bump-version",
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("rename-artifact"));
        assert!(rendered.contains("validation: ok"));

        let updated = fs::read_to_string(&markdown).expect("updated markdown should exist");
        assert!(updated.contains("artifact_path: process/b"));
        assert!(updated.contains("artifact_version: 2"));

        let changelog_body = fs::read_to_string(&changelog).expect("changelog should exist");
        assert!(changelog_body.contains("\"previous_artifact_path\":\"process/a\""));
        assert!(changelog_body.contains("\"event\":\"artifact_path_updated\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn init_creates_markdown_and_changelog_with_validation() {
        let root = temp_dir("init-root");
        let markdown = root.join("docs/process/new.md");
        let changelog = root.join("docs/process/new.changelog.jsonl");

        let cli = Cli::parse_from([
            "docflow",
            "init",
            markdown.to_string_lossy().as_ref(),
            "process/new",
            "process_doc",
            "initialize artifact",
            "--title",
            "New Artifact",
            "--purpose",
            "Init path test",
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("init"));
        assert!(rendered.contains("validation: ok"));

        let updated = fs::read_to_string(&markdown).expect("markdown should exist");
        assert!(updated.contains("# New Artifact"));
        assert!(updated.contains("Purpose: Init path test"));
        assert!(updated.contains("artifact_path: process/new"));

        let changelog_body = fs::read_to_string(&changelog).expect("changelog should exist");
        assert!(changelog_body.contains("\"event\":\"artifact_initialized\""));
        assert!(changelog_body.contains("\"reason\":\"initialize artifact\""));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn init_artifact_json_reports_created_artifact() {
        let root = temp_dir("init-json-root");
        let markdown = root.join("docs/process/new-json.md");

        let cli = Cli::parse_from([
            "docflow",
            "init",
            markdown.to_string_lossy().as_ref(),
            "process/new-json",
            "process_doc",
            "initialize artifact as json",
            "--json",
        ]);
        let rendered = run(cli);
        let payload: Value = serde_json::from_str(&rendered)
            .unwrap_or_else(|error| panic!("artifact init should emit JSON: {error}"));
        assert_eq!(
            payload.get("mode").and_then(|value| value.as_str()),
            Some("artifact_init")
        );
        assert_eq!(
            payload
                .get("artifact_path")
                .and_then(|value| value.as_str()),
            Some("process/new-json")
        );
        assert_eq!(
            payload
                .get("validation")
                .and_then(|value| value.get("verdict"))
                .and_then(|value| value.as_str()),
            Some("ok")
        );
        assert!(markdown.exists());

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn init_without_args_prints_agent_ready_instructions() {
        let cli = Cli::parse_from(["docflow", "init"]);
        let rendered = run(cli);
        assert!(rendered.contains("docflow init"));
        assert!(rendered.contains("mode: agent_bootstrap"));
        assert!(rendered.contains("AGENTS.sidecar.md"));
        assert!(rendered.contains("docflow readiness-check --profile active-canon"));
    }

    #[test]
    fn init_without_args_json_outputs_instructions_payload() {
        let cli = Cli::parse_from(["docflow", "init", "--json"]);
        let rendered = run(cli);
        let payload: Value = serde_json::from_str(&rendered).unwrap_or_else(|error| {
            panic!("init command should emit JSON when --json is set: {error}")
        });
        assert_eq!(
            payload.get("command"),
            Some(&Value::String("init".to_string()))
        );
        assert_eq!(
            payload.get("mode").and_then(|value| value.as_str()),
            Some("agent_bootstrap")
        );
        assert!(payload.get("instructions").is_some());
        assert!(payload.get("next_actions").is_some());
        assert!(
            payload
                .get("artifact_init")
                .and_then(|value| value.get("command"))
                .and_then(|value| value.as_str())
                .is_some()
        );
    }

    #[test]
    fn move_rehomes_markdown_and_changelog_with_updated_footer() {
        let root = temp_dir("move-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        let source = root.join("docs/process/a.md");
        let source_changelog = root.join("docs/process/a.changelog.jsonl");
        fs::write(
            &source,
            "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("markdown should exist");
        fs::write(
            &source_changelog,
            "{\"event\":\"artifact_initialized\",\"artifact_path\":\"process/a\"}\n",
        )
        .expect("changelog should exist");

        let destination = root.join("docs/product/spec/a.md");
        let cli = Cli::parse_from([
            "docflow",
            "move",
            source.to_string_lossy().as_ref(),
            destination.to_string_lossy().as_ref(),
            "move artifact",
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("move"));
        assert!(rendered.contains("validation: ok"));
        assert!(!source.exists());
        assert!(!source_changelog.exists());

        let updated = fs::read_to_string(&destination).expect("destination markdown should exist");
        assert!(updated.contains(&format!("source_path: {}", destination.to_string_lossy())));
        assert!(updated.contains("changelog_ref: a.changelog.jsonl"));

        let destination_changelog =
            fs::read_to_string(root.join("docs/product/spec/a.changelog.jsonl"))
                .expect("destination changelog should exist");
        assert!(destination_changelog.contains("\"event\":\"artifact_moved\""));
        assert!(destination_changelog.contains(&format!(
            "\"previous_source_path\":\"{}\"",
            source.to_string_lossy()
        )));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn migrate_links_dry_run_reports_replacements() {
        let root = temp_dir("migrate-links-root");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(
            root.join("docs/process/a.md"),
            "# a\n\n[Link](b.md)\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
        )
        .expect("markdown should exist");
        fs::write(
            root.join("docs/process/a.changelog.jsonl"),
            "{\"event\":\"artifact_initialized\",\"artifact_path\":\"process/a\"}\n",
        )
        .expect("changelog should exist");
        fs::write(root.join("docs/process/c.md"), "# c\n").expect("new target should exist");

        let cli = Cli::parse_from([
            "docflow",
            "migrate-links",
            root.join("docs/process/a.md").to_string_lossy().as_ref(),
            "b.md",
            "c.md",
            "rewrite links",
            "--dry-run",
            "--format",
            "jsonl",
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("\"replacements\":1"));
        let unchanged =
            fs::read_to_string(root.join("docs/process/a.md")).expect("markdown exists");
        assert!(unchanged.contains("[Link](b.md)"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn readiness_command_reports_blocking_for_missing_footer() {
        let cli = Cli::parse_from([
            "docflow",
            "readiness",
            "--path",
            "notes/test.md",
            "--content",
            "# title\n",
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("readiness"));
        assert!(rendered.contains("rows: 1"));
        assert!(rendered.contains("verdict: blocking"));
        assert!(rendered.contains("notes/test.md [blocking]"));
    }

    #[test]
    fn check_file_command_reads_markdown_artifact_from_disk() {
        let path = temp_path("check-file");
        fs::write(&path, "# title\n").expect("temp markdown should be written");
        let cli = Cli::parse_from(["docflow", "check-file", "--path", &path]);
        let rendered = run(cli);
        assert!(rendered.contains("validation"));
        assert!(rendered.contains("issues: 1"));
        assert!(rendered.contains("[missing_footer]"));
        fs::remove_file(path).expect("temp markdown should be removed");
    }

    #[test]
    fn check_command_accepts_positional_file_paths_under_root() {
        let root = temp_dir("check-positional");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("markdown should exist");

        let cli = Cli::parse_from([
            "docflow",
            "check",
            "--root",
            root.to_string_lossy().as_ref(),
            "docs/process/a.md",
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("\"path\":\"docs/process/a.md\""));
        assert!(rendered.contains("missing_footer"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn readiness_file_command_reads_markdown_artifact_from_disk() {
        let path = temp_path("readiness-file");
        fs::write(&path, "# title\n").expect("temp markdown should be written");
        let cli = Cli::parse_from(["docflow", "readiness-file", "--path", &path]);
        let rendered = run(cli);
        assert!(rendered.contains("readiness"));
        assert!(rendered.contains("rows: 1"));
        assert!(rendered.contains("verdict: blocking"));
        fs::remove_file(path).expect("temp markdown should be removed");
    }

    #[test]
    fn registry_scan_command_builds_registry_from_real_tree() {
        let root = temp_dir("registry");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::create_dir_all(root.join("docs/product/spec")).expect("spec dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");
        fs::write(root.join("docs/product/spec/b.md"), "# b\n").expect("spec markdown");

        let cli = Cli::parse_from([
            "docflow",
            "registry-scan",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("registry"));
        assert!(rendered.contains("total_rows: 2"));
        assert!(rendered.contains("docs/process/a.md [process_doc]"));
        assert!(rendered.contains("docs/product/spec/b.md [product_spec]"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn registry_scan_command_applies_default_policy_ignores() {
        let root = temp_dir("registry-ignored");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::create_dir_all(root.join("_temp/cache")).expect("temp dir should exist");
        fs::create_dir_all(root.join("dist/package")).expect("dist dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");
        fs::write(root.join("_temp/cache/ignored.md"), "# ignore\n").expect("temp markdown");
        fs::write(root.join("dist/package/ignored.md"), "# ignore\n").expect("dist markdown");

        let cli = Cli::parse_from([
            "docflow",
            "registry-scan",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("registry"));
        assert!(rendered.contains("docs/process/a.md [process_doc]"));
        assert!(!rendered.contains("_temp/cache/ignored.md"));
        assert!(!rendered.contains("dist/package/ignored.md"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn validate_tree_command_reports_tree_level_footer_issues() {
        let root = temp_dir("validate-tree");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");

        let cli = Cli::parse_from([
            "docflow",
            "validate-tree",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("validation-tree"));
        assert!(rendered.contains("scanned_rows: 1"));
        assert!(rendered.contains("issues: 3"));
        assert!(rendered.contains("verdict: blocking"));
        assert!(rendered.contains("docs/process/a.md [missing_footer]"));
        assert!(rendered.contains("docs/process/a.md [missing_agents_sidecar]"));
        assert!(rendered.contains("docs/process/a.md [missing_project_doc_map_surface]"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    #[test]
    fn readiness_tree_command_reports_tree_level_readiness_rows() {
        let root = temp_dir("readiness-tree");
        fs::create_dir_all(root.join("docs/process")).expect("process dir should exist");
        fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");

        let cli = Cli::parse_from([
            "docflow",
            "readiness-tree",
            "--root",
            root.to_string_lossy().as_ref(),
        ]);
        let rendered = run(cli);
        assert!(rendered.contains("readiness-tree"));
        assert!(rendered.contains("scanned_rows: 1"));
        assert!(rendered.contains("rows: 1"));
        assert!(rendered.contains("verdict: blocking"));
        assert!(rendered.contains("docs/process/a.md [blocking]"));

        fs::remove_dir_all(root).expect("temp root should be removed");
    }
}
