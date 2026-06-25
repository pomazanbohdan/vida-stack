use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn run_graph_transition_construction_is_owned_by_taskflow_authority() {
    let root = repo_root();
    let vida_run_graph = read(root.join("crates/vida/src/taskflow_run_graph.rs"));
    let authority = read(root.join("crates/taskflow-authority/src/run_graph_transition.rs"));

    for forbidden in [
        "fn governance_handoff",
        "struct RunGraphTransitionArgs",
        "fn run_graph_transition(",
    ] {
        assert!(
            !vida_run_graph.contains(forbidden),
            "vida run-graph surface must not own duplicate transition construction: {forbidden}"
        );
    }

    for required in [
        "pub enum RunGraphDispatchTargetFormat",
        "pub struct ReadyRunGraphTransitionInput",
        "pub fn ready_run_graph_transition",
        "pub fn run_graph_handoff",
    ] {
        assert!(
            authority.contains(required),
            "taskflow-authority must own run-graph transition construction: {required}"
        );
    }
}

#[test]
fn vida_surface_files_do_not_define_run_graph_transition_builders() {
    let src = repo_root().join("crates/vida/src");
    let mut offenders = Vec::new();
    for path in source_files(&src) {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if !(name.ends_with("_surface.rs") || name == "taskflow_run_graph.rs") {
            continue;
        }
        let body = read(&path);
        if body.contains("fn run_graph_transition(")
            || body.contains("struct RunGraphTransitionArgs")
            || body.contains("fn governance_handoff")
        {
            offenders.push(
                path.strip_prefix(repo_root())
                    .unwrap()
                    .display()
                    .to_string(),
            );
        }
    }

    assert!(
        offenders.is_empty(),
        "surface modules must delegate run-graph transition construction to taskflow-authority: {offenders:?}"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("vida-test-support is under crates")
        .to_path_buf()
}

fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_source_files(root, &mut files);
    files.sort();
    files
}

fn collect_source_files(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("source directory should be readable") {
        let entry = entry.expect("source entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_source_files(&path, files);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref())
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.as_ref().display()))
}
