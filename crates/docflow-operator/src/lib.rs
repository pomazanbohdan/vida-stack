use docflow_contracts::ReadinessRow;
use docflow_core::ReadinessVerdict;
use docflow_readiness::summarize_verdict;
use docflow_relations::RelationEdge;

pub fn render_overview(
    registry_count: usize,
    relation_count: usize,
    readiness_rows: &[ReadinessRow],
) -> String {
    let verdict = summarize_verdict(readiness_rows);
    format!(
        "docflow overview\n  registry_rows: {registry_count}\n  relation_edges: {relation_count}\n  readiness: {}",
        verdict_label(verdict)
    )
}

pub fn render_relation_summary(edges: &[RelationEdge]) -> String {
    format!("relations\n  total_edges: {}", edges.len())
}

pub fn render_artifact_impact(artifact: &str, source: &str, impacts: &[(&str, &str)]) -> String {
    let mut lines = vec![
        "artifact-impact".to_string(),
        format!("  artifact: {artifact}"),
        format!("  source: {source}"),
        format!("  impacts: {}", impacts.len()),
    ];
    for (path, reasons) in impacts {
        lines.push(format!("  impact: {path} [{reasons}]"));
    }
    lines.join("\n")
}

pub fn render_task_impact(
    task_id: &str,
    root: &str,
    touched: &[&str],
    impacts: &[(&str, &str, &str)],
) -> String {
    let mut lines = vec![
        "task-impact".to_string(),
        format!("  task_id: {task_id}"),
        format!("  root: {root}"),
        format!("  touched: {}", touched.len()),
        format!("  indirect_impacts: {}", impacts.len()),
    ];
    for path in touched {
        lines.push(format!("  touched_path: {path}"));
    }
    for (source_artifact, path, reasons) in impacts {
        lines.push(format!(
            "  indirect_impact: {path} <= {source_artifact} [{reasons}]"
        ));
    }
    lines.join("\n")
}

pub fn render_layer_status(
    layer: usize,
    current: &[(&str, &str)],
    adjacent: &[Vec<(&str, &str)>],
) -> String {
    let mut lines = vec!["layer-status".to_string(), format!("  layer: {layer}")];
    for (key, value) in current {
        lines.push(format!("  {key}: {value}"));
    }
    for row in adjacent {
        if row.is_empty() {
            continue;
        }
        let mut rendered = String::from("  adjacent:");
        for (idx, (key, value)) in row.iter().enumerate() {
            if idx == 0 {
                rendered.push_str(&format!(" {key}={value}"));
            } else {
                rendered.push_str(&format!(", {key}={value}"));
            }
        }
        lines.push(rendered);
    }
    lines.join("\n")
}

pub fn render_summary(
    root: &str,
    registry_count: usize,
    relation_count: usize,
    readiness_rows: &[ReadinessRow],
    type_counts: &[(&str, usize)],
) -> String {
    let mut lines = vec![
        "summary".to_string(),
        format!("  root: {root}"),
        format!("  registry_rows: {registry_count}"),
        format!("  relation_edges: {relation_count}"),
        format!(
            "  readiness: {}",
            verdict_label(summarize_verdict(readiness_rows))
        ),
    ];
    for (artifact_type, count) in type_counts {
        lines.push(format!("  type[{artifact_type}]: {count}"));
    }
    lines.join("\n")
}

fn verdict_label(verdict: ReadinessVerdict) -> &'static str {
    match verdict {
        ReadinessVerdict::Ok => "ok",
        ReadinessVerdict::Warning => "warning",
        ReadinessVerdict::Blocking => "blocking",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        render_artifact_impact, render_layer_status, render_overview, render_relation_summary,
        render_summary, render_task_impact,
    };
    use docflow_contracts::ReadinessRow;
    use docflow_core::{ArtifactPath, CheckedAt, ReadinessVerdict};
    use docflow_relations::RelationEdge;

    #[test]
    fn overview_renders_compact_docflow_summary() {
        let rows = vec![ReadinessRow {
            artifact_path: ArtifactPath("docs/process/a.md".into()),
            verdict: ReadinessVerdict::Warning,
            checked_at: CheckedAt::now_utc(),
        }];
        let rendered = render_overview(3, 2, &rows);
        assert!(rendered.contains("docflow overview"));
        assert!(rendered.contains("registry_rows: 3"));
        assert!(rendered.contains("readiness: warning"));
    }

    #[test]
    fn relation_summary_reports_edge_count() {
        let rendered = render_relation_summary(&[RelationEdge {
            from: ArtifactPath("docs/process/a.md".into()),
            to: ArtifactPath("docs/process/a.md".into()),
            relation_type: "artifact_identity".into(),
        }]);
        assert_eq!(rendered, "relations\n  total_edges: 1");
    }

    #[test]
    fn artifact_impact_renders_compact_operator_surface() {
        let rendered = render_artifact_impact(
            "process/a",
            "artifact",
            &[("docs/process/b.md", "footer_ref,markdown_link")],
        );
        assert!(rendered.contains("artifact-impact"));
        assert!(rendered.contains("artifact: process/a"));
        assert!(rendered.contains("impacts: 1"));
        assert!(rendered.contains("impact: docs/process/b.md [footer_ref,markdown_link]"));
    }

    #[test]
    fn task_impact_renders_compact_operator_surface() {
        let rendered = render_task_impact(
            "vida-stack-r1-b14",
            "/tmp/root",
            &["docs/process/a.md"],
            &[("process/a", "docs/process/b.md", "footer_ref")],
        );
        assert!(rendered.contains("task-impact"));
        assert!(rendered.contains("task_id: vida-stack-r1-b14"));
        assert!(rendered.contains("touched: 1"));
        assert!(rendered.contains("indirect_impacts: 1"));
        assert!(rendered.contains("touched_path: docs/process/a.md"));
        assert!(rendered.contains("indirect_impact: docs/process/b.md <= process/a [footer_ref]"));
    }

    #[test]
    fn layer_status_renders_current_and_adjacent_rows() {
        let rendered = render_layer_status(
            6,
            &[("Layer name", "Canonical Operator"), ("Status", "✅")],
            &[vec![
                ("position", "previous"),
                ("Layer name", "Canonical Relations"),
            ]],
        );
        assert!(rendered.contains("layer-status"));
        assert!(rendered.contains("layer: 6"));
        assert!(rendered.contains("Layer name: Canonical Operator"));
        assert!(rendered.contains("Status: ✅"));
        assert!(rendered.contains("adjacent: position=previous, Layer name=Canonical Relations"));
    }

    #[test]
    fn summary_renders_counts_and_verdict() {
        let readiness = vec![ReadinessRow {
            artifact_path: ArtifactPath("docs/process/a.md".into()),
            verdict: ReadinessVerdict::Blocking,
            checked_at: CheckedAt::now_utc(),
        }];
        let rendered = render_summary(
            "/tmp/root",
            2,
            2,
            &readiness,
            &[("process_doc", 1), ("product_spec", 1)],
        );
        assert!(rendered.contains("summary"));
        assert!(rendered.contains("root: /tmp/root"));
        assert!(rendered.contains("registry_rows: 2"));
        assert!(rendered.contains("relation_edges: 2"));
        assert!(rendered.contains("readiness: blocking"));
        assert!(rendered.contains("type[process_doc]: 1"));
    }
}
