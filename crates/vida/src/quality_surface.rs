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
}
