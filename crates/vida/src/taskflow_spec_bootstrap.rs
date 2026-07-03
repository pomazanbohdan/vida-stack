use std::fs;
use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use docflow_cli::Cli as DocflowCli;
use time::format_description::well_known::Rfc3339;

use crate::runtime_contract_vocab::TASK_CLASS_IMPLEMENTATION;
use crate::state_store::{
    CreateTaskRequest, StateStore, TaskExecutionSemantics, TaskPlannerMetadata, UpdateTaskRequest,
};
use crate::taskflow_task_bridge::task_record_json;

struct TaskCreationArgs<'a> {
    store: &'a StateStore,
    project_root: &'a Path,
    task_id: &'a str,
    title: &'a str,
    issue_type: &'a str,
    status: &'a str,
    parent_id: Option<&'a str>,
    labels: &'a [&'a str],
    description: Option<&'a str>,
    execution_semantics: TaskExecutionSemantics,
    planner_metadata: TaskPlannerMetadata,
}

fn create_task_if_missing_with_store(
    args: TaskCreationArgs<'_>,
) -> Result<(serde_json::Value, bool), String> {
    let TaskCreationArgs {
        store,
        project_root,
        task_id,
        title,
        issue_type,
        status,
        parent_id,
        labels,
        description,
        execution_semantics,
        planner_metadata,
    } = args;

    if let Ok(existing) = crate::block_on_state_store(store.show_task(task_id)) {
        let backfill_planner =
            should_backfill_planner_metadata(&existing.planner_metadata, &planner_metadata);
        let backfill_semantics = should_backfill_execution_semantics(
            &existing.execution_semantics,
            &execution_semantics,
        );
        if backfill_planner || backfill_semantics {
            let empty_labels = Vec::<String>::new();
            let updated =
                crate::block_on_state_store(
                    store.update_task(UpdateTaskRequest {
                        task_id,
                        title: None,
                        status: None,
                        priority: None,
                        notes: None,
                        description: None,
                        parent_id: None,
                        add_labels: &empty_labels,
                        remove_labels: &empty_labels,
                        set_labels: None,
                        execution_mode: backfill_semantics
                            .then_some(execution_semantics.execution_mode.as_deref()),
                        order_bucket: backfill_semantics
                            .then_some(execution_semantics.order_bucket.as_deref()),
                        parallel_group: backfill_semantics
                            .then_some(execution_semantics.parallel_group.as_deref()),
                        conflict_domain: backfill_semantics
                            .then_some(execution_semantics.conflict_domain.as_deref()),
                        planner_metadata: backfill_planner.then_some(planner_metadata),
                    }),
                )?;
            return Ok((task_record_json(&updated), false));
        }
        return Ok((task_record_json(&existing), false));
    }
    let label_rows = labels
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    let description_value = description.unwrap_or_default();
    let source_repo = project_root.display().to_string();
    match crate::block_on_state_store(store.create_task(CreateTaskRequest {
        task_id,
        title,
        display_id: None,
        description: description_value,
        issue_type,
        status,
        priority: 2,
        parent_id,
        labels: &label_rows,
        execution_semantics,
        planner_metadata,
        created_by: "vida taskflow",
        source_repo: &source_repo,
    })) {
        Ok(task) => Ok((task_record_json(&task), true)),
        Err(error) if error.contains("task already exists") => {
            let existing = crate::block_on_state_store(store.show_task(task_id))?;
            Ok((task_record_json(&existing), false))
        }
        Err(error) => Err(error),
    }
}

fn should_backfill_execution_semantics(
    existing: &TaskExecutionSemantics,
    expected: &TaskExecutionSemantics,
) -> bool {
    let existing_missing = existing.execution_mode.is_none()
        && existing.order_bucket.is_none()
        && existing.parallel_group.is_none()
        && existing.conflict_domain.is_none();
    let expected_present = expected.execution_mode.is_some()
        || expected.order_bucket.is_some()
        || expected.parallel_group.is_some()
        || expected.conflict_domain.is_some();
    existing_missing && expected_present
}

fn work_packet_execution_semantics(
    epic_task_id: &str,
    task_id: &str,
    packet_label: &str,
) -> TaskExecutionSemantics {
    let execution_mode = if packet_label == "work-pool-pack" {
        "container_only"
    } else {
        "parallel_safe"
    };
    TaskExecutionSemantics {
        execution_mode: Some(execution_mode.to_string()),
        order_bucket: Some(epic_task_id.to_string()),
        parallel_group: Some(packet_label.to_string()),
        conflict_domain: Some(task_id.to_string()),
    }
}

fn should_backfill_planner_metadata(
    existing: &TaskPlannerMetadata,
    desired: &TaskPlannerMetadata,
) -> bool {
    existing.owned_paths.is_empty()
        && existing.acceptance_targets.is_empty()
        && existing.proof_targets.is_empty()
        && (!desired.owned_paths.is_empty()
            || !desired.acceptance_targets.is_empty()
            || !desired.proof_targets.is_empty())
}

fn work_packet_planner_metadata(
    request_text: &str,
    tracked: &serde_json::Value,
) -> TaskPlannerMetadata {
    let design_doc_path = tracked["design_doc_path"].as_str();
    let mut owned_paths = crate::runtime_dispatch_packets::delivery_packet_owned_paths(
        TASK_CLASS_IMPLEMENTATION,
        request_text,
        design_doc_path,
    );
    owned_paths.retain(|path| {
        !crate::runtime_dispatch_packets::is_runtime_consumption_fallback_owned_path(path)
    });
    if owned_paths.is_empty() {
        return TaskPlannerMetadata::default();
    }
    TaskPlannerMetadata {
        acceptance_targets: vec![format!(
            "bounded implementation touches only {}",
            owned_paths.join(", ")
        )],
        proof_targets: vec!["cargo test -p vida".to_string()],
        risk: Some("medium".to_string()),
        estimate: Some("bounded".to_string()),
        lane_hint: Some("worker".to_string()),
        owned_paths,
    }
}

pub(crate) fn run_docflow_cli_command(
    project_root: &Path,
    args: &[String],
) -> Result<String, String> {
    run_docflow_cli_command_with_exit(project_root, args).map(|result| result.output)
}

pub(crate) fn run_docflow_cli_command_with_exit(
    project_root: &Path,
    args: &[String],
) -> Result<docflow_cli::RunResult, String> {
    let previous = std::env::var_os("VIDA_ROOT");
    std::env::set_var("VIDA_ROOT", project_root);
    let argv = std::iter::once("docflow".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    let result = DocflowCli::try_parse_from(argv)
        .map_err(|error| error.to_string())
        .map(docflow_cli::run_with_exit);
    match previous {
        Some(value) => std::env::set_var("VIDA_ROOT", value),
        None => std::env::remove_var("VIDA_ROOT"),
    }
    result
}

pub(crate) async fn run_taskflow_bootstrap_spec(args: &[String]) -> ExitCode {
    let as_json = args.iter().any(|arg| arg == "--json");
    let request_text = args
        .iter()
        .skip(1)
        .filter(|arg| arg.as_str() != "--json")
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if request_text.is_empty() {
        eprintln!("Usage: vida taskflow bootstrap-spec <request_text> [--json]");
        return ExitCode::from(2);
    }

    let project_root = match crate::resolve_repo_root() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    let state_dir = crate::taskflow_task_bridge::proxy_state_dir();
    let store = match crate::StateStore::open_existing(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    let role_selection =
        match crate::build_runtime_lane_selection_with_store(&store, &request_text).await {
            Ok(selection) => selection,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        };
    if role_selection.execution_plan["status"] != "design_first" {
        eprintln!(
            "Spec bootstrap is allowed only for design-first feature requests. Use `vida taskflow consume final <request> --json` to inspect the routed intake posture first."
        );
        return ExitCode::from(1);
    }

    let tracked = &role_selection.execution_plan["tracked_flow_bootstrap"];
    let payload = match execute_taskflow_bootstrap_spec_with_store(
        &project_root,
        &store,
        &request_text,
        tracked,
    ) {
        Ok(payload) => payload,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload)
                .expect("spec bootstrap payload should render as json")
        );
    } else {
        crate::print_surface_header(crate::RenderMode::Plain, "vida taskflow bootstrap-spec");
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "feature slug",
            payload["feature_slug"]
                .as_str()
                .unwrap_or("feature-request"),
        );
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "epic",
            payload["epic"]["task_id"]
                .as_str()
                .unwrap_or("feature-request"),
        );
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "spec task",
            payload["spec_task"]["task_id"]
                .as_str()
                .unwrap_or("feature-request-spec"),
        );
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "design doc",
            payload["design_doc"]["path"]
                .as_str()
                .unwrap_or("docs/product/spec/feature-request-design.md"),
        );
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "receipt path",
            payload["receipt_path"].as_str().unwrap_or(""),
        );
        if let Some(note) = payload["next"]["plan_note"].as_str() {
            crate::print_surface_line(crate::RenderMode::Plain, "first step", note);
        }
        if let Some(command) = payload["next"]["finalize_command"].as_str() {
            crate::print_surface_line(crate::RenderMode::Plain, "finalize design", command);
        }
        if let Some(command) = payload["next"]["check_command"].as_str() {
            crate::print_surface_line(crate::RenderMode::Plain, "check design", command);
        }
        if let Some(command) = payload["next"]["close_spec_task_command"].as_str() {
            crate::print_surface_line(crate::RenderMode::Plain, "close spec task", command);
        }
        if let Some(command) = payload["next"]["work_pool_ensure_command"].as_str() {
            crate::print_surface_line(crate::RenderMode::Plain, "next work-pool command", command);
        }
        if let Some(command) = payload["next"]["dev_task_ensure_command"].as_str() {
            crate::print_surface_line(crate::RenderMode::Plain, "next dev command", command);
        }
    }

    ExitCode::SUCCESS
}

fn docflow_output_is_ok(output: &str) -> bool {
    !output.contains("verdict: blocking")
        && !output.contains("validation: blocking")
        && !output.contains("error:")
        && !output.contains("❌ BLOCKING:")
}
fn ensure_product_spec_index(project_root: &Path) -> Result<bool, String> {
    let index_path = project_root.join(crate::DEFAULT_PROJECT_PRODUCT_SPEC_INDEX);
    reject_product_spec_index_symlink(&index_path)?;
    if !index_path.is_file() {
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
        }
        fs::write(
            &index_path,
            crate::init_surfaces::render_project_product_spec_index(),
        )
        .map_err(|error| format!("Failed to write {}: {error}", index_path.display()))?;
        return Ok(true);
    }
    Ok(false)
}

fn ensure_product_spec_index_entry(
    project_root: &Path,
    design_doc_path: &str,
    feature_slug: &str,
) -> Result<bool, String> {
    let index_path = project_root.join(crate::DEFAULT_PROJECT_PRODUCT_SPEC_INDEX);
    reject_product_spec_index_symlink(&index_path)?;
    let mut content = fs::read_to_string(&index_path)
        .map_err(|error| format!("Failed to read {}: {error}", index_path.display()))?;
    if content.contains(design_doc_path) {
        return Ok(false);
    }
    if !content.contains("## Generated Feature Design Docs") {
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str("\n## Generated Feature Design Docs\n");
    }
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!("- `{design_doc_path}` ({feature_slug})\n"));
    fs::write(&index_path, content)
        .map_err(|error| format!("Failed to write {}: {error}", index_path.display()))?;
    Ok(true)
}

fn reject_product_spec_index_symlink(index_path: &Path) -> Result<(), String> {
    let Ok(metadata) = fs::symlink_metadata(index_path) else {
        return Ok(());
    };
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Refusing to use product spec index symlink {}",
            index_path.display()
        ));
    }
    Ok(())
}

fn ensure_project_docs_sidecar_pointers(project_root: &Path) -> Result<bool, String> {
    let sidecar_path = project_root.join("AGENTS.sidecar.md");
    if !sidecar_path.is_file() {
        return Err(format!(
            "Missing project docs sidecar: {}",
            sidecar_path.display()
        ));
    }
    let mut content = fs::read_to_string(&sidecar_path)
        .map_err(|error| format!("Failed to read {}: {error}", sidecar_path.display()))?;
    let mut changed = false;
    for pointer in [
        "docs/project-root-map.md",
        "docs/process/documentation-tooling-map.md",
    ] {
        if !content.contains(pointer) {
            if !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(&format!("- `{pointer}`\n"));
            changed = true;
        }
    }
    if changed {
        fs::write(&sidecar_path, content)
            .map_err(|error| format!("Failed to write {}: {error}", sidecar_path.display()))?;
    }
    Ok(changed)
}

struct SpecBootstrapReceiptArgs<'a> {
    project_root: &'a Path,
    request: &'a str,
    feature_slug: &'a str,
    epic_task_id: &'a str,
    spec_task_id: &'a str,
    design_doc_path: &'a str,
    changed_files: &'a [String],
}

fn write_spec_bootstrap_receipt(args: SpecBootstrapReceiptArgs<'_>) -> Result<String, String> {
    let project_root = args.project_root;
    let receipts_dir = project_root.join(".vida/receipts");
    fs::create_dir_all(&receipts_dir)
        .map_err(|error| format!("Failed to create {}: {error}", receipts_dir.display()))?;
    let now = time::OffsetDateTime::now_utc();
    let recorded_at = now
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let receipt_path = receipts_dir.join(format!("spec-bootstrap-{}.json", now.unix_timestamp()));
    let receipt = serde_json::json!({
        "receipt_kind": "spec_bootstrap",
        "recorded_at": recorded_at,
        "surface": "vida taskflow bootstrap-spec",
        "project_root": project_root.display().to_string(),
        "request": args.request,
        "feature_slug": args.feature_slug,
        "epic_task_id": args.epic_task_id,
        "spec_task_id": args.spec_task_id,
        "design_doc_path": args.design_doc_path,
        "changed_files": args.changed_files,
        "next_step": "finalize the design document through vida docflow, then close the spec task and continue through the next TaskFlow packet",
    });
    let body =
        serde_json::to_string_pretty(&receipt).expect("spec bootstrap receipt should render");
    fs::write(&receipt_path, &body)
        .map_err(|error| format!("Failed to write {}: {error}", receipt_path.display()))?;
    fs::write(
        project_root.join(crate::SPEC_BOOTSTRAP_RECEIPT_LATEST),
        &body,
    )
    .map_err(|error| {
        format!(
            "Failed to write {}: {error}",
            project_root
                .join(crate::SPEC_BOOTSTRAP_RECEIPT_LATEST)
                .display()
        )
    })?;
    Ok(receipt_path
        .strip_prefix(project_root)
        .unwrap_or(&receipt_path)
        .display()
        .to_string())
}

pub(crate) fn execute_taskflow_bootstrap_spec_with_store(
    project_root: &Path,
    store: &StateStore,
    request_text: &str,
    tracked: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    fn required_bool(value: &serde_json::Value, path: &str) -> Result<bool, String> {
        value
            .as_bool()
            .ok_or_else(|| format!("missing required tracked bootstrap evidence `{path}`"))
    }
    fn required_str<'a>(value: &'a serde_json::Value, path: &str) -> Result<&'a str, String> {
        value
            .as_str()
            .filter(|entry| !entry.trim().is_empty())
            .ok_or_else(|| format!("missing required tracked bootstrap evidence `{path}`"))
    }

    if !tracked.is_object() {
        return Err(
            "missing required tracked bootstrap evidence `tracked_flow_bootstrap`".to_string(),
        );
    }

    let required = required_bool(&tracked["required"], "tracked_flow_bootstrap.required")?;
    if !required {
        return Err(
            "tracked bootstrap evidence `tracked_flow_bootstrap.required` must be true".to_string(),
        );
    }
    let status = required_str(&tracked["status"], "tracked_flow_bootstrap.status")?;
    if status != "pending" {
        return Err(format!(
            "tracked bootstrap evidence `tracked_flow_bootstrap.status` must be `pending`, got `{status}`"
        ));
    }
    let bootstrap_command = required_str(
        &tracked["bootstrap_command"],
        "tracked_flow_bootstrap.bootstrap_command",
    )?;
    if !bootstrap_command.contains("vida taskflow bootstrap-spec")
        || !bootstrap_command.contains("--json")
    {
        return Err(
            "tracked bootstrap evidence `tracked_flow_bootstrap.bootstrap_command` is invalid"
                .to_string(),
        );
    }

    let feature_slug = required_str(
        &tracked["feature_slug"],
        "tracked_flow_bootstrap.feature_slug",
    )?;
    let design_doc_path = required_str(
        &tracked["design_doc_path"],
        "tracked_flow_bootstrap.design_doc_path",
    )?;
    let artifact_path = required_str(
        &tracked["design_artifact_path"],
        "tracked_flow_bootstrap.design_artifact_path",
    )?;
    let epic_task_id = required_str(
        &tracked["epic"]["task_id"],
        "tracked_flow_bootstrap.epic.task_id",
    )?;
    let epic_title = required_str(
        &tracked["epic"]["title"],
        "tracked_flow_bootstrap.epic.title",
    )?;
    let spec_task_id = required_str(
        &tracked["spec_task"]["task_id"],
        "tracked_flow_bootstrap.spec_task.task_id",
    )?;
    let spec_title = required_str(
        &tracked["spec_task"]["title"],
        "tracked_flow_bootstrap.spec_task.title",
    )?;
    let finalize_command = required_str(
        &tracked["docflow"]["finalize_command"],
        "tracked_flow_bootstrap.docflow.finalize_command",
    )?;
    let check_command = required_str(
        &tracked["docflow"]["check_command"],
        "tracked_flow_bootstrap.docflow.check_command",
    )?;
    let close_spec_task_command = required_str(
        &tracked["spec_task"]["close_command"],
        "tracked_flow_bootstrap.spec_task.close_command",
    )?;
    let work_pool_task_id = required_str(
        &tracked["work_pool_task"]["task_id"],
        "tracked_flow_bootstrap.work_pool_task.task_id",
    )?;
    let work_pool_title = required_str(
        &tracked["work_pool_task"]["title"],
        "tracked_flow_bootstrap.work_pool_task.title",
    )?;
    let work_pool_create_command = required_str(
        &tracked["work_pool_task"]["create_command"],
        "tracked_flow_bootstrap.work_pool_task.create_command",
    )?;
    let work_pool_ensure_command = required_str(
        &tracked["work_pool_task"]["ensure_command"],
        "tracked_flow_bootstrap.work_pool_task.ensure_command",
    )?;
    let dev_task_create_command = required_str(
        &tracked["dev_task"]["create_command"],
        "tracked_flow_bootstrap.dev_task.create_command",
    )?;
    let dev_task_ensure_command = required_str(
        &tracked["dev_task"]["ensure_command"],
        "tracked_flow_bootstrap.dev_task.ensure_command",
    )?;

    let mut changed_files = Vec::new();
    let (_, epic_created) = create_task_if_missing_with_store(TaskCreationArgs {
        store,
        project_root,
        task_id: epic_task_id,
        title: epic_title,
        issue_type: "epic",
        status: "open",
        parent_id: None,
        labels: &["feature-request", "spec-first"],
        description: Some(request_text),
        execution_semantics: TaskExecutionSemantics::default(),
        planner_metadata: TaskPlannerMetadata::default(),
    })?;
    if epic_created {
        changed_files.push(format!("taskflow:{epic_task_id}"));
    }

    let (_, spec_created) = create_task_if_missing_with_store(TaskCreationArgs {
        store,
        project_root,
        task_id: spec_task_id,
        title: spec_title,
        issue_type: "task",
        status: "open",
        parent_id: Some(epic_task_id),
        labels: &["spec-pack", "documentation"],
        description: Some("bounded design/spec packet for the feature request"),
        execution_semantics: TaskExecutionSemantics::default(),
        planner_metadata: TaskPlannerMetadata::default(),
    })?;
    if spec_created {
        changed_files.push(format!("taskflow:{spec_task_id}"));
    }

    let (_, work_pool_created) = create_task_if_missing_with_store(TaskCreationArgs {
        store,
        project_root,
        task_id: work_pool_task_id,
        title: work_pool_title,
        issue_type: "task",
        status: "open",
        parent_id: Some(epic_task_id),
        labels: &["work-pool-pack"],
        description: Some("tracked work-pool packet created during design-first bootstrap"),
        execution_semantics: work_packet_execution_semantics(
            epic_task_id,
            work_pool_task_id,
            "work-pool-pack",
        ),
        planner_metadata: TaskPlannerMetadata::default(),
    })?;
    if work_pool_created {
        changed_files.push(format!("taskflow:{work_pool_task_id}"));
    }

    match ensure_project_docs_sidecar_pointers(project_root) {
        Ok(true) => changed_files.push("AGENTS.sidecar.md".to_string()),
        Ok(false) => {}
        Err(error) => return Err(error),
    }

    match ensure_product_spec_index(project_root) {
        Ok(true) => changed_files.push(crate::DEFAULT_PROJECT_PRODUCT_SPEC_INDEX.to_string()),
        Ok(false) => {}
        Err(error) => return Err(error),
    }
    match ensure_product_spec_index_entry(project_root, design_doc_path, feature_slug) {
        Ok(true)
            if !changed_files
                .iter()
                .any(|path| path == crate::DEFAULT_PROJECT_PRODUCT_SPEC_INDEX) =>
        {
            changed_files.push(crate::DEFAULT_PROJECT_PRODUCT_SPEC_INDEX.to_string());
        }
        Ok(_) => {}
        Err(error) => return Err(error),
    }

    let design_doc_abs = project_root.join(design_doc_path);
    let design_doc_created = if design_doc_abs.exists() {
        false
    } else {
        let init_output = run_docflow_cli_command(
            project_root,
            &[
                "init".to_string(),
                design_doc_path.to_string(),
                artifact_path.to_string(),
                "product_spec".to_string(),
                "initialize bounded feature design".to_string(),
            ],
        )?;
        if !docflow_output_is_ok(&init_output) {
            return Err(init_output);
        }
        changed_files.push(design_doc_path.to_string());
        true
    };

    let check_output = run_docflow_cli_command(
        project_root,
        &[
            "check-file".to_string(),
            "--path".to_string(),
            design_doc_path.to_string(),
        ],
    )?;
    if !docflow_output_is_ok(&check_output) {
        return Err(check_output);
    }

    let receipt_path = write_spec_bootstrap_receipt(SpecBootstrapReceiptArgs {
        project_root,
        request: request_text,
        feature_slug,
        epic_task_id,
        spec_task_id,
        design_doc_path,
        changed_files: &changed_files,
    })?;

    Ok(serde_json::json!({
        "surface": "vida taskflow bootstrap-spec",
        "status": "pass",
        "admission": {
            "status": "admitted",
            "admitted": true,
            "consumed_evidence": [
                "tracked_flow_bootstrap.required",
                "tracked_flow_bootstrap.status",
                "tracked_flow_bootstrap.bootstrap_command",
                "tracked_flow_bootstrap.feature_slug",
                "tracked_flow_bootstrap.design_doc_path",
                "tracked_flow_bootstrap.design_artifact_path",
                "tracked_flow_bootstrap.epic.task_id",
                "tracked_flow_bootstrap.spec_task.task_id",
                "tracked_flow_bootstrap.docflow.finalize_command",
                "tracked_flow_bootstrap.docflow.check_command",
                "tracked_flow_bootstrap.spec_task.close_command",
            "tracked_flow_bootstrap.work_pool_task.task_id",
            "tracked_flow_bootstrap.work_pool_task.title",
            "tracked_flow_bootstrap.work_pool_task.create_command",
            "tracked_flow_bootstrap.work_pool_task.ensure_command",
            "tracked_flow_bootstrap.dev_task.create_command",
            "tracked_flow_bootstrap.dev_task.ensure_command",
        ],
        },
        "request": request_text,
        "feature_slug": feature_slug,
        "epic": {
            "task_id": epic_task_id,
            "created": epic_created,
        },
        "spec_task": {
            "task_id": spec_task_id,
            "created": spec_created,
        },
        "work_pool_task": {
            "task_id": work_pool_task_id,
            "created": work_pool_created,
            "preopened_before_spec_close": true,
        },
        "design_doc": {
            "path": design_doc_path,
            "created": design_doc_created,
            "registered_in": [crate::DEFAULT_PROJECT_PRODUCT_SPEC_INDEX],
        },
        "next": {
            "plan_note": "publish a concise execution plan before mutating the design document or dispatching write-producing work",
            "finalize_command": finalize_command,
            "check_command": check_command,
            "close_spec_task_command": close_spec_task_command,
            "work_pool_create_command": work_pool_create_command,
            "work_pool_ensure_command": work_pool_ensure_command,
            "dev_task_create_command": dev_task_create_command,
            "dev_task_ensure_command": dev_task_ensure_command,
        },
        "receipt_path": receipt_path,
        "changed_files": changed_files,
    }))
}

pub(crate) fn execute_work_packet_create_with_store(
    project_root: &Path,
    store: &StateStore,
    request_text: &str,
    tracked: &serde_json::Value,
    packet_key: &str,
) -> Result<serde_json::Value, String> {
    let tracked = if tracked[packet_key]["task_id"].as_str().is_some() {
        tracked.clone()
    } else {
        crate::build_design_first_tracked_flow_bootstrap(request_text)
    };
    let epic_task_id = tracked["epic"]["task_id"]
        .as_str()
        .unwrap_or("feature-request");
    let epic_title = tracked["epic"]["title"].as_str().unwrap_or("Feature epic");
    let task = &tracked[packet_key];
    let task_id = task["task_id"]
        .as_str()
        .ok_or_else(|| format!("missing task_id for `{packet_key}`"))?;
    let title = task["title"]
        .as_str()
        .ok_or_else(|| format!("missing title for `{packet_key}`"))?;
    let packet_label = if packet_key == "work_pool_task" {
        "work-pool-pack"
    } else {
        "dev-pack"
    };
    let packet_description = if packet_key == "work_pool_task" {
        "tracked work-pool packet created from runtime consumption"
    } else {
        "tracked dev packet created from runtime consumption"
    };
    let mut changed_files = Vec::new();

    let (_, epic_created) = create_task_if_missing_with_store(TaskCreationArgs {
        store,
        project_root,
        task_id: epic_task_id,
        title: epic_title,
        issue_type: "epic",
        status: "open",
        parent_id: None,
        labels: &["feature-request"],
        description: Some("tracked feature epic for runtime-consumption dispatch"),
        execution_semantics: TaskExecutionSemantics::default(),
        planner_metadata: TaskPlannerMetadata::default(),
    })?;
    if epic_created {
        changed_files.push(format!("taskflow:{epic_task_id}"));
    }

    let (_, packet_created) = create_task_if_missing_with_store(TaskCreationArgs {
        store,
        project_root,
        task_id,
        title,
        issue_type: "task",
        status: "open",
        parent_id: Some(epic_task_id),
        labels: &[packet_label],
        description: Some(packet_description),
        execution_semantics: work_packet_execution_semantics(epic_task_id, task_id, packet_label),
        planner_metadata: work_packet_planner_metadata(request_text, &tracked),
    })?;
    if packet_created {
        changed_files.push(format!("taskflow:{task_id}"));
    }

    Ok(serde_json::json!({
        "surface": "vida task ensure",
        "status": "pass",
        "packet_key": packet_key,
        "epic": {
            "task_id": epic_task_id,
            "created": epic_created,
        },
        "task": {
            "task_id": task_id,
            "created": packet_created,
            "reused_existing": !packet_created,
            "label": packet_label,
        },
        "changed_files": changed_files,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn spec_bootstrap_receipt_writes_files_to_temp_root() {
        let unique_root = std::env::temp_dir().join(format!(
            "vida-spec-bootstrap-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should flow")
                .as_nanos()
        ));
        fs::create_dir_all(&unique_root).expect("temporary project root should be creatable");
        let changed_files = vec!["taskflow:test-epic".to_string()];
        let args = SpecBootstrapReceiptArgs {
            project_root: &unique_root,
            request: "test-request",
            feature_slug: "test-feature",
            epic_task_id: "EPIC-001",
            spec_task_id: "SPEC-001",
            design_doc_path: "docs/product/spec/preview.md",
            changed_files: &changed_files,
        };
        let receipt_path =
            write_spec_bootstrap_receipt(args).expect("receipt writer should succeed");
        let absolute_receipt = unique_root.join(&receipt_path);
        assert!(
            absolute_receipt.exists(),
            "expected receipt at {}",
            absolute_receipt.display()
        );
        assert!(
            unique_root
                .join(crate::SPEC_BOOTSTRAP_RECEIPT_LATEST)
                .exists(),
            ".vida receipt latest pointer should exist"
        );
        fs::remove_dir_all(&unique_root).expect("cleanup should succeed");
    }

    #[test]
    fn work_packet_create_backfills_dev_task_owned_paths_from_explicit_request() {
        let unique_root = std::env::temp_dir().join(format!(
            "vida-work-packet-metadata-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should flow")
                .as_nanos()
        ));
        fs::create_dir_all(&unique_root).expect("temporary project root should be creatable");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(StateStore::open(
                unique_root.join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let tracked = serde_json::json!({
            "epic": {
                "task_id": "feature-x",
                "title": "Feature X"
            },
            "dev_task": {
                "task_id": "feature-x-dev",
                "title": "Dev pack: Feature X"
            }
        });

        execute_work_packet_create_with_store(
            &unique_root,
            &store,
            "continue implementation",
            &tracked,
            "dev_task",
        )
        .expect("initial dev packet should be created");
        let initial = runtime
            .block_on(store.show_task("feature-x-dev"))
            .expect("initial dev task should load");
        assert!(initial.planner_metadata.owned_paths.is_empty());
        assert_eq!(
            initial.execution_semantics.execution_mode.as_deref(),
            Some("parallel_safe")
        );
        assert_eq!(
            initial.execution_semantics.order_bucket.as_deref(),
            Some("feature-x")
        );
        assert_eq!(
            initial.execution_semantics.parallel_group.as_deref(),
            Some("dev-pack")
        );
        assert_eq!(
            initial.execution_semantics.conflict_domain.as_deref(),
            Some("feature-x-dev")
        );

        execute_work_packet_create_with_store(
            &unique_root,
            &store,
            "Continue implementation in crates/vida/src/taskflow_spec_bootstrap.rs and crates/vida/src/runtime_dispatch_state.rs",
            &tracked,
            "dev_task",
        )
        .expect("existing dev packet should be metadata-backfilled");

        let updated = runtime
            .block_on(store.show_task("feature-x-dev"))
            .expect("updated dev task should load");
        assert_eq!(updated.planner_metadata.owned_paths.len(), 2);
        assert!(
            updated
                .planner_metadata
                .owned_paths
                .contains(&"crates/vida/src/taskflow_spec_bootstrap.rs".to_string())
        );
        assert!(
            updated
                .planner_metadata
                .owned_paths
                .contains(&"crates/vida/src/runtime_dispatch_state.rs".to_string())
        );
        assert!(!updated.planner_metadata.acceptance_targets.is_empty());
        assert!(!updated.planner_metadata.proof_targets.is_empty());
        assert_eq!(
            updated.execution_semantics.execution_mode.as_deref(),
            Some("parallel_safe")
        );
        assert_eq!(
            updated.execution_semantics.order_bucket.as_deref(),
            Some("feature-x")
        );

        fs::remove_dir_all(&unique_root).expect("cleanup should succeed");
    }

    #[test]
    fn work_packet_create_uses_container_only_for_work_pool() {
        let unique_root = std::env::temp_dir().join(format!(
            "vida-work-packet-work-pool-semantics-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should flow")
                .as_nanos()
        ));
        fs::create_dir_all(&unique_root).expect("temporary project root should be creatable");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(StateStore::open(
                unique_root.join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let tracked = serde_json::json!({
            "epic": {
                "task_id": "feature-x",
                "title": "Feature X"
            },
            "work_pool_task": {
                "task_id": "feature-x-work-pool",
                "title": "Work-pool pack: Feature X"
            }
        });

        execute_work_packet_create_with_store(
            &unique_root,
            &store,
            "shape the work pool",
            &tracked,
            "work_pool_task",
        )
        .expect("work-pool packet should be created");

        let work_pool = runtime
            .block_on(store.show_task("feature-x-work-pool"))
            .expect("work-pool task should load");
        assert_eq!(
            work_pool.execution_semantics.execution_mode.as_deref(),
            Some("container_only")
        );
        assert_eq!(
            work_pool.execution_semantics.order_bucket.as_deref(),
            Some("feature-x")
        );
        assert_eq!(
            work_pool.execution_semantics.parallel_group.as_deref(),
            Some("work-pool-pack")
        );
        assert_eq!(
            work_pool.execution_semantics.conflict_domain.as_deref(),
            Some("feature-x-work-pool")
        );

        fs::remove_dir_all(&unique_root).expect("cleanup should succeed");
    }
}
