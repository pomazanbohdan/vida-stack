use vida_test_support::domain_conformance::{
    DOMAIN_CONFORMANCE_SCHEMA_VERSION, assert_domain_conformance,
};

#[test]
fn taskflow_domain_conformance_corpus_passes_without_state_store() {
    let report = assert_domain_conformance();

    assert_eq!(report.schema_version, DOMAIN_CONFORMANCE_SCHEMA_VERSION);
    assert_eq!(report.scenario_count(), 9);
    assert_eq!(
        report.covered_semantic_areas(),
        vec!["continuation", "run_graph", "task_lifecycle"]
    );
}

#[test]
fn architecture_lint_rejects_surface_ready_handoff_construction() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve from crate manifest dir");
    let source_path = root
        .join("crates")
        .join("vida")
        .join("src")
        .join("state_store_run_graph_summary.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("run-graph summary source should read");

    let offenders = source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let constructs_ready_handoff = line
                .contains("status.handoff_state = format!(\"awaiting_")
                || (line.contains("status.resume_target = format!(\"dispatch.")
                    && line.contains("_lane"));
            constructs_ready_handoff.then_some(format!("{}:{}", index + 1, line.trim()))
        })
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "ready handoff construction belongs in taskflow-authority::run_graph_transition: {offenders:#?}"
    );
    assert!(
        source.contains("ready_run_graph_transition(input)"),
        "vida surface must remain a field adapter over taskflow-authority ready transition construction"
    );
}
