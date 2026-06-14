//! Task dependency command helpers.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDependencyBulkEdge {
    pub issue_id: String,
    pub depends_on_id: String,
    pub edge_type: String,
}

pub fn parse_task_dependency_bulk_edge(raw: &str) -> Result<TaskDependencyBulkEdge, String> {
    let parts = raw.split(':').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!(
            "invalid bulk dependency edge `{raw}`; expected issue_id:depends_on_id:edge_type"
        ));
    }
    if parts.iter().any(|part| part.is_empty()) {
        return Err(format!(
            "invalid bulk dependency edge `{raw}`; expected non-empty issue_id, depends_on_id, and edge_type"
        ));
    }
    Ok(TaskDependencyBulkEdge {
        issue_id: parts[0].to_string(),
        depends_on_id: parts[1].to_string(),
        edge_type: parts[2].to_string(),
    })
}

pub fn parse_task_dependency_bulk_edges<'a>(
    raw_edges: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<TaskDependencyBulkEdge>, String> {
    raw_edges
        .into_iter()
        .map(parse_task_dependency_bulk_edge)
        .collect()
}

pub fn task_dependency_bulk_edge_lines<'a>(
    lines: impl IntoIterator<Item = &'a str>,
) -> Vec<String> {
    lines
        .into_iter()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        TaskDependencyBulkEdge, parse_task_dependency_bulk_edge, parse_task_dependency_bulk_edges,
        task_dependency_bulk_edge_lines,
    };

    #[test]
    fn parses_valid_bulk_dependency_edge() {
        assert_eq!(
            parse_task_dependency_bulk_edge("task-a:task-b:blocks").unwrap(),
            TaskDependencyBulkEdge {
                issue_id: "task-a".to_string(),
                depends_on_id: "task-b".to_string(),
                edge_type: "blocks".to_string(),
            }
        );
    }

    #[test]
    fn trims_bulk_dependency_edge_fields() {
        assert_eq!(
            parse_task_dependency_bulk_edge(" task-a : task-b : parent-child ").unwrap(),
            TaskDependencyBulkEdge {
                issue_id: "task-a".to_string(),
                depends_on_id: "task-b".to_string(),
                edge_type: "parent-child".to_string(),
            }
        );
    }

    #[test]
    fn rejects_malformed_bulk_dependency_edge_arity() {
        assert!(parse_task_dependency_bulk_edge("task-a:task-b").is_err());
        assert!(parse_task_dependency_bulk_edge("task-a:task-b:blocks:extra").is_err());
    }

    #[test]
    fn rejects_empty_bulk_dependency_edge_fields() {
        assert!(parse_task_dependency_bulk_edge(":task-b:blocks").is_err());
        assert!(parse_task_dependency_bulk_edge("task-a: :blocks").is_err());
        assert!(parse_task_dependency_bulk_edge("task-a:task-b:").is_err());
    }

    #[test]
    fn filters_blank_and_comment_bulk_dependency_edge_lines() {
        assert_eq!(
            task_dependency_bulk_edge_lines([
                "",
                "   ",
                "# comment",
                " task-a : task-b : blocks ",
                "task-c:task-d:parent-child",
            ]),
            vec![
                "task-a : task-b : blocks".to_string(),
                "task-c:task-d:parent-child".to_string(),
            ]
        );
    }

    #[test]
    fn parses_bulk_dependency_edges() {
        assert_eq!(
            parse_task_dependency_bulk_edges(["task-a:task-b:blocks", "task-c:task-d:unblocks"])
                .unwrap()
                .len(),
            2
        );
    }
}
