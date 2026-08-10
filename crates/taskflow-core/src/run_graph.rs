//! Run-graph module skeleton for future TaskFlow core extraction.

pub mod closure;
pub mod continuation;
pub mod model;
pub mod projections;
pub mod recovery;
pub mod stale;
pub mod status;

#[cfg(test)]
mod tests {
    #[test]
    fn run_graph_module_identity_is_stable() {
        assert!(module_path!().ends_with("::run_graph::tests"));
    }
}
