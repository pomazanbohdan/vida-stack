pub(crate) mod bootstrap_sources;
pub(crate) mod materialization;

#[cfg(test)]
mod tests {
    use super::{bootstrap_sources, materialization};
    use std::path::Path;

    #[test]
    fn init_submodules_preserve_bootstrap_and_materialization_roots() {
        let root = Path::new("runtime-root");
        assert_eq!(
            bootstrap_sources::installed_runtime_source_root_candidates(root),
            vec![root.join("current"), root.to_path_buf()]
        );
        assert_eq!(
            materialization::default_init_instruction_bundle_source_roots(root),
            (
                root.join("vida/config/instructions/bundles/framework-source"),
                root.join("vida/config/instructions/bundles/framework-memory-source")
            )
        );
    }
}
