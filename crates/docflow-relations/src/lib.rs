use docflow_contracts::RegistryRow;
use docflow_core::ArtifactPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationEdge {
    pub from: ArtifactPath,
    pub to: ArtifactPath,
    pub relation_type: String,
}

pub fn artifact_identity_edges(rows: &[RegistryRow]) -> Vec<RelationEdge> {
    let mut edges: Vec<RelationEdge> = rows
        .iter()
        .map(|row| RelationEdge {
            from: row.artifact_path.clone(),
            to: row.artifact_path.clone(),
            relation_type: "artifact_identity".into(),
        })
        .collect();
    edges.sort_by(|left, right| left.from.0.cmp(&right.from.0));
    edges
}

pub fn reverse_reference_edges(edges: &[RelationEdge]) -> Vec<RelationEdge> {
    let mut reversed: Vec<RelationEdge> = edges
        .iter()
        .map(|edge| RelationEdge {
            from: edge.to.clone(),
            to: edge.from.clone(),
            relation_type: format!("reverse_{}", edge.relation_type),
        })
        .collect();
    reversed.sort_by(|left, right| {
        left.from
            .0
            .cmp(&right.from.0)
            .then(left.to.0.cmp(&right.to.0))
    });
    reversed
}

#[cfg(test)]
mod tests {
    use super::{artifact_identity_edges, reverse_reference_edges};
    use docflow_contracts::RegistryRow;
    use docflow_core::ArtifactPath;

    #[test]
    fn artifact_identity_edges_are_sorted() {
        let rows = vec![
            RegistryRow {
                artifact_path: ArtifactPath("docs/product/spec/b.md".into()),
                artifact_type: "product_spec".into(),
            },
            RegistryRow {
                artifact_path: ArtifactPath("docs/process/a.md".into()),
                artifact_type: "process_doc".into(),
            },
        ];
        let edges = artifact_identity_edges(&rows);
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].from.0, "docs/process/a.md");
        assert_eq!(edges[0].relation_type, "artifact_identity");
    }

    #[test]
    fn reverse_edges_swap_direction_and_prefix_type() {
        let edges = artifact_identity_edges(&[RegistryRow {
            artifact_path: ArtifactPath("docs/process/a.md".into()),
            artifact_type: "process_doc".into(),
        }]);
        let reversed = reverse_reference_edges(&edges);
        assert_eq!(reversed.len(), 1);
        assert_eq!(reversed[0].from.0, "docs/process/a.md");
        assert_eq!(reversed[0].to.0, "docs/process/a.md");
        assert_eq!(reversed[0].relation_type, "reverse_artifact_identity");
    }
}
