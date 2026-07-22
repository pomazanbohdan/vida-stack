use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::Value;

use crate::release1_operator_output::{
    finalize_operator_surface_verdict, RELEASE1_OPERATOR_CONTRACT_SPEC,
};
use crate::QualityCommand;

const QUALITY_GATE_SURFACE: &str = "vida quality gate";

#[derive(Debug, Default)]
struct CoverageFile {
    path: String,
    found_lines: u64,
    hit_lines: u64,
}

#[derive(Debug, Default)]
struct CoverageSummary {
    source: Option<String>,
    total_lines: u64,
    covered_lines: u64,
    percent: Option<f64>,
    threshold: f64,
    additional_covered_lines_needed: u64,
    top_uncovered_changed_files: Vec<Value>,
    status: &'static str,
}

#[derive(Debug, Clone)]
struct CrapEntry {
    file: String,
    function: String,
    line: u64,
    crate_name: String,
    crap: f64,
    cyclomatic: f64,
    coverage: f64,
}

#[derive(Debug, Default)]
struct CrapSummary {
    source: Option<String>,
    baseline_source: Option<String>,
    status: &'static str,
    count_gt_30: usize,
    count_gt_100: usize,
    count_gt_1000: usize,
    top_hotspots: Vec<Value>,
    per_crate_hotspots: Vec<Value>,
    touched_hotspots: Vec<Value>,
    worsened_hotspots: Vec<Value>,
    task_exception_present: bool,
}

pub(crate) async fn run_quality(args: crate::QualityArgs) -> ExitCode {
    match args.command {
        QualityCommand::Gate(command) => run_quality_gate(command),
    }
}

fn run_quality_gate(command: crate::QualityGateArgs) -> ExitCode {
    let payload = build_quality_gate_payload(&command);
    if command.json {
        crate::print_json_pretty(&payload);
    } else {
        println!(
            "{}",
            operator_output::toon_report::render_value(
                QUALITY_GATE_SURFACE,
                quality_gate_toon_payload(&payload),
            )
        );
    }

    if payload["status"].as_str() == Some("pass") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn build_quality_gate_payload(command: &crate::QualityGateArgs) -> Value {
    let project_root = command
        .project_root
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let project_root = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.clone());

    let changed_files = git_changed_files(&project_root);
    let codegen_dirty_files = changed_files
        .iter()
        .filter(|path| generated_or_codegen_path(path))
        .cloned()
        .collect::<Vec<_>>();
    let coverage = coverage_summary(
        &project_root,
        command.coverage_file.as_ref(),
        command.coverage_threshold,
        &changed_files,
    );
    let crap = crap_summary(
        &project_root,
        command.crap_file.as_ref(),
        command.crap_baseline_file.as_ref(),
        &changed_files,
        &command.task_exception_note,
    );

    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    if !command.prepush {
        blocker_codes.push("missing_prepush_gate_scope".to_string());
        next_actions.push(
            "Run `vida quality gate --prepush` to evaluate the pre-push quality gate advisor."
                .to_string(),
        );
    }
    if !codegen_dirty_files.is_empty() {
        blocker_codes.push("codegen_dirty_files".to_string());
        next_actions.push(format!(
            "Review generated/codegen files, rerun the project code generator if needed, then stage the intended files: {}.",
            git_add_command(&codegen_dirty_files)
        ));
    }
    if coverage.additional_covered_lines_needed > 0 {
        blocker_codes.push("coverage_below_threshold".to_string());
        next_actions.push(format!(
            "Add at least {} covered line(s) or adjust tests for the top uncovered changed files before rerunning the pre-push gate.",
            coverage.additional_covered_lines_needed
        ));
    }
    if !crap.worsened_hotspots.is_empty() {
        blocker_codes.push("crap_gt_1000_growth".to_string());
        next_actions.push(
            "Reduce the worsened CRAP>1000 functions or update the reviewed CRAP baseline after an accepted refactor proof.".to_string(),
        );
    }
    if !crap.touched_hotspots.is_empty() && !crap.task_exception_present {
        blocker_codes.push("touched_crap_hotspots_without_exception".to_string());
        next_actions.push(
            "Add coverage/refactor proof for touched CRAP>1000 functions or pass --task-exception-note with the TaskFlow exception id.".to_string(),
        );
    }
    blocker_codes.sort();
    blocker_codes.dedup();

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    let suggested_action = if command.advise {
        suggested_action(&blocker_codes, &codegen_dirty_files, &coverage)
    } else {
        None
    };
    let artifact_refs = serde_json::json!({
        "surface": QUALITY_GATE_SURFACE,
        "project_root": project_root.display().to_string(),
        "coverage_file": coverage.source,
        "crap_file": crap.source,
        "crap_baseline_file": crap.baseline_source,
        "affected_paths": codegen_dirty_files,
    });
    let verdict = finalize_operator_surface_verdict(
        &RELEASE1_OPERATOR_CONTRACT_SPEC,
        status,
        blocker_codes,
        next_actions,
        artifact_refs.clone(),
    );
    let coverage_payload = serde_json::json!({
        "status": coverage.status,
        "source": coverage.source,
        "coverage_percent": coverage.percent,
        "coverage_threshold": coverage.threshold,
        "covered_lines": coverage.covered_lines,
        "total_lines": coverage.total_lines,
        "additional_covered_lines_needed": coverage.additional_covered_lines_needed,
        "top_uncovered_changed_files": coverage.top_uncovered_changed_files,
    });
    let crap_payload = serde_json::json!({
        "status": crap.status,
        "source": crap.source,
        "baseline_source": crap.baseline_source,
        "count_gt_30": crap.count_gt_30,
        "count_gt_100": crap.count_gt_100,
        "count_gt_1000": crap.count_gt_1000,
        "top_hotspots": crap.top_hotspots,
        "per_crate_hotspots": crap.per_crate_hotspots,
        "touched_hotspots": crap.touched_hotspots,
        "worsened_hotspots": crap.worsened_hotspots,
        "task_exception_present": crap.task_exception_present,
    });

    serde_json::json!({
        "surface": QUALITY_GATE_SURFACE,
        "status": verdict.status,
        "blocker_codes": verdict.blocker_codes,
        "next_actions": verdict.next_actions,
        "artifact_refs": artifact_refs,
        "shared_fields": verdict.shared_fields,
        "operator_contracts": verdict.operator_contracts,
        "prepush": command.prepush,
        "advise": command.advise,
        "codegen_dirty_files": codegen_dirty_files,
        "coverage": coverage_payload,
        "crap": crap_payload,
        "coverage_percent": coverage.percent,
        "coverage_threshold": coverage.threshold,
        "additional_covered_lines_needed": coverage.additional_covered_lines_needed,
        "top_uncovered_changed_files": coverage.top_uncovered_changed_files,
        "suggested_action": suggested_action,
    })
}

fn quality_gate_toon_payload(payload: &Value) -> Value {
    serde_json::json!({
        "status": payload["status"],
        "blocker_codes": payload["blocker_codes"],
        "codegen_dirty_count": payload["codegen_dirty_files"].as_array().map(Vec::len).unwrap_or(0),
        "coverage_percent": payload["coverage_percent"],
        "coverage_threshold": payload["coverage_threshold"],
        "additional_covered_lines_needed": payload["additional_covered_lines_needed"],
        "top_uncovered_changed_files": payload["top_uncovered_changed_files"],
        "crap": payload["crap"],
        "suggested_action": payload["suggested_action"],
        "next_actions": payload["next_actions"],
    })
}

fn git_changed_files(project_root: &Path) -> Vec<String> {
    let Ok(output) = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["status", "--porcelain", "-uall"])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.get(3..).or_else(|| line.get(2..)))
        .flat_map(|path| path.split(" -> ").last())
        .map(normalize_repo_path)
        .filter(|path| !path.is_empty())
        .collect()
}

fn generated_or_codegen_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".g.dart")
        || lower.ends_with(".freezed.dart")
        || lower.ends_with(".gr.dart")
        || lower.contains("/generated/")
        || lower.contains("\\generated\\")
        || lower.contains("generated")
        || lower.contains("codegen")
}

fn coverage_summary(
    project_root: &Path,
    coverage_file: Option<&PathBuf>,
    threshold: f64,
    changed_files: &[String],
) -> CoverageSummary {
    let coverage_path = coverage_file
        .cloned()
        .or_else(|| default_coverage_file(project_root));
    let Some(coverage_path) = coverage_path else {
        return CoverageSummary {
            threshold,
            status: "not_found",
            ..CoverageSummary::default()
        };
    };
    let Ok(body) = std::fs::read_to_string(&coverage_path) else {
        return CoverageSummary {
            source: Some(coverage_path.display().to_string()),
            threshold,
            status: "unreadable",
            ..CoverageSummary::default()
        };
    };
    let files = parse_lcov(&body);
    let total_lines = files.iter().map(|file| file.found_lines).sum::<u64>();
    let covered_lines = files.iter().map(|file| file.hit_lines).sum::<u64>();
    let percent = (total_lines > 0).then(|| covered_lines as f64 * 100.0 / total_lines as f64);
    let additional_covered_lines_needed = percent
        .filter(|percent| *percent < threshold)
        .map(|_| ((threshold / 100.0) * total_lines as f64).ceil() as u64)
        .map(|required| required.saturating_sub(covered_lines))
        .unwrap_or(0);
    let top_uncovered_changed_files = top_uncovered_changed_files(&files, changed_files);
    CoverageSummary {
        source: Some(coverage_path.display().to_string()),
        total_lines,
        covered_lines,
        percent,
        threshold,
        additional_covered_lines_needed,
        top_uncovered_changed_files,
        status: if total_lines > 0 { "loaded" } else { "empty" },
    }
}

fn default_coverage_file(project_root: &Path) -> Option<PathBuf> {
    ["coverage/lcov.info", "lcov.info"]
        .iter()
        .map(|path| project_root.join(path))
        .find(|path| path.is_file())
}

fn parse_lcov(body: &str) -> Vec<CoverageFile> {
    let mut files = Vec::new();
    let mut current = CoverageFile::default();
    for line in body.lines() {
        if let Some(source) = line.strip_prefix("SF:") {
            if !current.path.is_empty() {
                files.push(current);
            }
            current = CoverageFile {
                path: normalize_repo_path(source),
                ..CoverageFile::default()
            };
        } else if let Some(found) = line.strip_prefix("LF:") {
            current.found_lines = found.trim().parse().unwrap_or(0);
        } else if let Some(hit) = line.strip_prefix("LH:") {
            current.hit_lines = hit.trim().parse().unwrap_or(0);
        } else if line.trim() == "end_of_record" && !current.path.is_empty() {
            files.push(current);
            current = CoverageFile::default();
        }
    }
    if !current.path.is_empty() {
        files.push(current);
    }
    files
}

fn top_uncovered_changed_files(files: &[CoverageFile], changed_files: &[String]) -> Vec<Value> {
    let changed = changed_files
        .iter()
        .map(|path| (normalize_repo_path(path), true))
        .collect::<BTreeMap<_, _>>();
    let mut rows = files
        .iter()
        .filter(|file| changed.is_empty() || changed.contains_key(&file.path))
        .map(|file| {
            let uncovered = file.found_lines.saturating_sub(file.hit_lines);
            (uncovered, file)
        })
        .filter(|(uncovered, _)| *uncovered > 0)
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.path.cmp(&right.1.path))
    });
    rows.into_iter()
        .take(5)
        .map(|(uncovered_lines, file)| {
            serde_json::json!({
                "path": file.path,
                "uncovered_lines": uncovered_lines,
                "covered_lines": file.hit_lines,
                "total_lines": file.found_lines,
            })
        })
        .collect()
}

fn crap_summary(
    project_root: &Path,
    crap_file: Option<&PathBuf>,
    baseline_file: Option<&PathBuf>,
    changed_files: &[String],
    exception_notes: &[String],
) -> CrapSummary {
    let crap_path = crap_file
        .cloned()
        .or_else(|| default_crap_file(project_root));
    let task_exception_present = exception_notes.iter().any(|note| !note.trim().is_empty());
    let Some(crap_path) = crap_path else {
        return CrapSummary {
            status: "not_found",
            task_exception_present,
            ..CrapSummary::default()
        };
    };
    let Ok(body) = std::fs::read_to_string(&crap_path) else {
        return CrapSummary {
            source: Some(crap_path.display().to_string()),
            status: "unreadable",
            task_exception_present,
            ..CrapSummary::default()
        };
    };
    let entries = parse_crap_entries(project_root, &body);
    if entries.is_empty() {
        return CrapSummary {
            source: Some(crap_path.display().to_string()),
            status: if body.trim().is_empty() {
                "empty"
            } else {
                "invalid_or_empty"
            },
            task_exception_present,
            ..CrapSummary::default()
        };
    }

    let baseline = baseline_file
        .and_then(|path| std::fs::read_to_string(path).ok().map(|body| (path, body)))
        .map(|(path, body)| {
            (
                path.display().to_string(),
                parse_crap_entries(project_root, &body)
                    .into_iter()
                    .map(|entry| (crap_entry_key(&entry), entry))
                    .collect::<BTreeMap<_, _>>(),
            )
        });
    summarize_crap_entries(
        Some(crap_path.display().to_string()),
        baseline.as_ref().map(|(path, _)| path.clone()),
        entries,
        baseline.map(|(_, entries)| entries).unwrap_or_default(),
        changed_files,
        task_exception_present,
    )
}

fn default_crap_file(project_root: &Path) -> Option<PathBuf> {
    [".vida/tmp/workspace-crap.json"]
        .iter()
        .map(|path| project_root.join(path))
        .find(|path| path.is_file())
}

fn parse_crap_entries(project_root: &Path, body: &str) -> Vec<CrapEntry> {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    value
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let file = entry.get("file")?.as_str()?;
            let function = entry.get("function")?.as_str()?;
            Some(CrapEntry {
                file: normalize_project_path(project_root, file),
                function: function.to_string(),
                line: value_as_u64(entry.get("line")).unwrap_or(0),
                crate_name: entry
                    .get("crate")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                crap: value_as_f64(entry.get("crap")).unwrap_or(0.0),
                cyclomatic: value_as_f64(entry.get("cyclomatic")).unwrap_or(0.0),
                coverage: value_as_f64(entry.get("coverage")).unwrap_or(0.0),
            })
        })
        .collect()
}

fn summarize_crap_entries(
    source: Option<String>,
    baseline_source: Option<String>,
    mut entries: Vec<CrapEntry>,
    baseline_entries: BTreeMap<String, CrapEntry>,
    changed_files: &[String],
    task_exception_present: bool,
) -> CrapSummary {
    entries.sort_by(|left, right| {
        right
            .crap
            .partial_cmp(&left.crap)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.file.cmp(&right.file))
            .then_with(|| left.function.cmp(&right.function))
    });
    let changed = changed_files
        .iter()
        .map(|path| (normalize_repo_path(path), true))
        .collect::<BTreeMap<_, _>>();
    let mut per_crate = BTreeMap::<String, (usize, usize, usize)>::new();
    for entry in &entries {
        let bucket = per_crate.entry(entry.crate_name.clone()).or_default();
        if entry.crap > 30.0 {
            bucket.0 += 1;
        }
        if entry.crap > 100.0 {
            bucket.1 += 1;
        }
        if entry.crap > 1000.0 {
            bucket.2 += 1;
        }
    }
    let mut per_crate_hotspots = per_crate
        .into_iter()
        .map(|(crate_name, (gt_30, gt_100, gt_1000))| (gt_1000, gt_100, gt_30, crate_name))
        .collect::<Vec<_>>();
    per_crate_hotspots.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| left.3.cmp(&right.3))
    });

    let top_hotspots = entries
        .iter()
        .take(10)
        .map(|entry| crap_entry_to_value(entry, None))
        .collect::<Vec<_>>();
    let touched_hotspots = entries
        .iter()
        .filter(|entry| entry.crap > 1000.0 && changed.contains_key(&entry.file))
        .take(10)
        .map(|entry| crap_entry_to_value(entry, None))
        .collect::<Vec<_>>();
    let worsened_hotspots = entries
        .iter()
        .filter_map(|entry| {
            let previous = baseline_entries.get(&crap_entry_key(entry))?;
            (entry.crap > 1000.0 && entry.crap > previous.crap)
                .then(|| crap_entry_to_value(entry, Some(previous.crap)))
        })
        .take(10)
        .collect::<Vec<_>>();

    CrapSummary {
        source,
        baseline_source,
        status: "loaded",
        count_gt_30: entries.iter().filter(|entry| entry.crap > 30.0).count(),
        count_gt_100: entries.iter().filter(|entry| entry.crap > 100.0).count(),
        count_gt_1000: entries.iter().filter(|entry| entry.crap > 1000.0).count(),
        top_hotspots,
        per_crate_hotspots: per_crate_hotspots
            .into_iter()
            .map(|(gt_1000, gt_100, gt_30, crate_name)| {
                serde_json::json!({
                    "crate": crate_name,
                    "count_gt_30": gt_30,
                    "count_gt_100": gt_100,
                    "count_gt_1000": gt_1000,
                })
            })
            .collect(),
        touched_hotspots,
        worsened_hotspots,
        task_exception_present,
    }
}

fn crap_entry_key(entry: &CrapEntry) -> String {
    format!("{}:{}:{}", entry.file, entry.function, entry.line)
}

fn crap_entry_to_value(entry: &CrapEntry, previous_crap: Option<f64>) -> Value {
    let delta = previous_crap.map(|previous| entry.crap - previous);
    serde_json::json!({
        "file": entry.file,
        "function": entry.function,
        "line": entry.line,
        "crate": entry.crate_name,
        "crap": entry.crap,
        "previous_crap": previous_crap,
        "crap_delta": delta,
        "cyclomatic": entry.cyclomatic,
        "coverage": entry.coverage,
    })
}

fn value_as_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|raw| raw.parse().ok()))
    })
}

fn value_as_u64(value: Option<&Value>) -> Option<u64> {
    value.and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|raw| u64::try_from(raw).ok()))
            .or_else(|| value.as_str().and_then(|raw| raw.parse::<u64>().ok()))
    })
}

fn suggested_action(
    blocker_codes: &[String],
    codegen_dirty_files: &[String],
    coverage: &CoverageSummary,
) -> Option<String> {
    if blocker_codes.is_empty() {
        return Some(
            "No pre-push quality remediation is required from the available evidence.".to_string(),
        );
    }
    let mut parts = Vec::new();
    if !codegen_dirty_files.is_empty() {
        parts.push(format!(
            "stage intended generated files with `{}`",
            git_add_command(codegen_dirty_files)
        ));
    }
    if coverage.additional_covered_lines_needed > 0 {
        parts.push(format!(
            "add at least {} covered line(s) across the top uncovered changed files",
            coverage.additional_covered_lines_needed
        ));
    }
    (!parts.is_empty()).then(|| parts.join("; "))
}

fn git_add_command(paths: &[String]) -> String {
    let files = paths
        .iter()
        .map(|path| crate::shell_quote(path))
        .collect::<Vec<_>>()
        .join(" ");
    format!("git add {files}")
}

fn normalize_repo_path(path: &str) -> String {
    path.trim()
        .trim_matches('"')
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn normalize_project_path(project_root: &Path, path: &str) -> String {
    let normalized = normalize_repo_path(path);
    let root = normalize_repo_path(&project_root.display().to_string());
    let root_without_verbatim = root.strip_prefix("//?/").unwrap_or(&root);
    for candidate in [&root, root_without_verbatim] {
        let prefix = format!("{candidate}/");
        if normalized
            .to_ascii_lowercase()
            .starts_with(&prefix.to_ascii_lowercase())
        {
            return normalized[prefix.len()..].to_string();
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcov_summary_reports_additional_covered_lines_needed() {
        let files = parse_lcov("SF:src/a.rs\nLF:10\nLH:8\nend_of_record\n");
        let rows = top_uncovered_changed_files(&files, &["src/a.rs".to_string()]);
        assert_eq!(rows[0]["uncovered_lines"], 2);
    }

    #[test]
    fn generated_path_classifier_covers_common_codegen_outputs() {
        assert!(generated_or_codegen_path("lib/foo.g.dart"));
        assert!(generated_or_codegen_path("src/generated/client.rs"));
        assert!(!generated_or_codegen_path("src/handwritten.rs"));
    }

    #[test]
    fn normalize_project_path_strips_windows_verbatim_project_root() {
        let root = PathBuf::from(r"\\?\C:\project\vida-stack");

        assert_eq!(
            normalize_project_path(&root, r"C:\project\vida-stack\crates\vida\src\hot.rs"),
            "crates/vida/src/hot.rs"
        );
        assert_eq!(
            normalize_project_path(&root, r"\\?\C:\project\vida-stack\crates\vida\src\hot.rs"),
            "crates/vida/src/hot.rs"
        );
    }

    #[test]
    fn crap_summary_reports_buckets_per_crate_and_touched_hotspots() {
        let root = PathBuf::from("C:/project/vida-stack");
        let entries = parse_crap_entries(
            &root,
            r#"{"entries":[
                {"file":"C:\\project\\vida-stack\\crates\\vida\\src\\hot.rs","function":"hot","line":7,"crate":"vida","crap":1200.0,"cyclomatic":80.0,"coverage":12.5},
                {"file":"crates/vida/src/warm.rs","function":"warm","line":3,"crate":"vida","crap":125.0,"cyclomatic":20.0,"coverage":50.0},
                {"file":"crates/docflow/src/cool.rs","function":"cool","line":5,"crate":"docflow","crap":31.0,"cyclomatic":10.0,"coverage":90.0}
            ]}"#,
        );

        let summary = summarize_crap_entries(
            Some("workspace-crap.json".to_string()),
            None,
            entries,
            BTreeMap::new(),
            &["crates/vida/src/hot.rs".to_string()],
            false,
        );

        assert_eq!(summary.count_gt_30, 3);
        assert_eq!(summary.count_gt_100, 2);
        assert_eq!(summary.count_gt_1000, 1);
        assert_eq!(summary.touched_hotspots[0]["function"], "hot");
        assert_eq!(summary.per_crate_hotspots[0]["crate"], "vida");
    }

    #[test]
    fn crap_summary_reports_worsened_baseline_hotspots() {
        let current = vec![CrapEntry {
            file: "crates/vida/src/hot.rs".to_string(),
            function: "hot".to_string(),
            line: 7,
            crate_name: "vida".to_string(),
            crap: 1200.0,
            cyclomatic: 80.0,
            coverage: 12.5,
        }];
        let previous = CrapEntry {
            crap: 950.0,
            ..current[0].clone()
        };
        let baseline = [(crap_entry_key(&previous), previous)]
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        let summary = summarize_crap_entries(None, None, current, baseline, &Vec::new(), false);

        assert_eq!(summary.worsened_hotspots[0]["function"], "hot");
        assert_eq!(summary.worsened_hotspots[0]["previous_crap"], 950.0);
        assert_eq!(summary.worsened_hotspots[0]["crap_delta"], 250.0);
    }
}
