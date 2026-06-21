//! Task graph algorithm helpers.

use std::collections::BTreeMap;

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectedDependencyAnalysis {
    pub topological_order: Vec<String>,
    pub cycle_node_id: Option<String>,
}

#[must_use]
pub fn analyze_directed_dependencies<'a>(
    nodes: impl IntoIterator<Item = &'a str>,
    edges: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> DirectedDependencyAnalysis {
    let mut node_ids = nodes
        .into_iter()
        .map(str::trim)
        .filter(|node_id| !node_id.is_empty())
        .collect::<Vec<_>>();
    node_ids.sort();
    node_ids.dedup();

    let mut graph = DiGraph::<&str, ()>::new();
    let mut node_indices = BTreeMap::<&str, NodeIndex>::new();
    for node_id in node_ids {
        let index = graph.add_node(node_id);
        node_indices.insert(node_id, index);
    }

    for (issue_id, depends_on_id) in edges {
        let (Some(issue_index), Some(depends_on_index)) = (
            node_indices.get(issue_id.trim()).copied(),
            node_indices.get(depends_on_id.trim()).copied(),
        ) else {
            continue;
        };
        graph.add_edge(issue_index, depends_on_index, ());
    }

    match toposort(&graph, None) {
        Ok(order) => DirectedDependencyAnalysis {
            topological_order: order
                .into_iter()
                .filter_map(|index| graph.node_weight(index).copied())
                .map(ToString::to_string)
                .collect(),
            cycle_node_id: None,
        },
        Err(cycle) => DirectedDependencyAnalysis {
            topological_order: Vec::new(),
            cycle_node_id: graph
                .node_weight(cycle.node_id())
                .copied()
                .map(ToString::to_string),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::analyze_directed_dependencies;

    #[test]
    fn graph_analysis_reports_topological_order_for_acyclic_dependencies() {
        let analysis = analyze_directed_dependencies(
            ["task-a", "task-b", "task-c"],
            [("task-c", "task-b"), ("task-b", "task-a")],
        );

        assert!(analysis.cycle_node_id.is_none());
        assert_eq!(analysis.topological_order.len(), 3);
    }

    #[test]
    fn graph_analysis_reports_cycle_node_for_cyclic_dependencies() {
        let analysis = analyze_directed_dependencies(
            ["task-a", "task-b"],
            [("task-a", "task-b"), ("task-b", "task-a")],
        );

        assert!(matches!(
            analysis.cycle_node_id.as_deref(),
            Some("task-a" | "task-b")
        ));
        assert!(analysis.topological_order.is_empty());
    }
}
