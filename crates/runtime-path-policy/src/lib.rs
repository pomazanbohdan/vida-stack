pub mod atomic_write;
pub mod bounded_json;
pub mod safe_path;
pub mod size_limits;
pub mod state_root;
pub mod symlink_policy;

pub use atomic_write::{
    AtomicReplaceLimit, DEFAULT_ATOMIC_REPLACE_MAX_BYTES, HARD_ATOMIC_REPLACE_MAX_BYTES,
    atomic_replace_bounded, atomic_replace_bounded_from_file, atomic_replace_bounded_from_reader,
    atomic_replace_bounded_with_limit,
};

pub use safe_path::{
    ArtifactPathKind, ExistingRegularFile, NewStateOutputPath, PathPolicyError,
    existing_regular_file_under_root, new_output_path_under_root, path_contains_dot_segment,
    read_bounded_text_file_under_root,
};
pub use state_root::StateRoot;

#[cfg(test)]
mod tests {
    use super::{
        AtomicReplaceLimit, DEFAULT_ATOMIC_REPLACE_MAX_BYTES, HARD_ATOMIC_REPLACE_MAX_BYTES,
    };

    #[test]
    fn public_path_policy_surface_reexports_atomic_limit_contract() {
        assert_eq!(
            DEFAULT_ATOMIC_REPLACE_MAX_BYTES,
            HARD_ATOMIC_REPLACE_MAX_BYTES
        );
        assert_eq!(
            AtomicReplaceLimit::new(HARD_ATOMIC_REPLACE_MAX_BYTES + 1).max_bytes(),
            HARD_ATOMIC_REPLACE_MAX_BYTES
        );
        assert_eq!(
            AtomicReplaceLimit::new(1024).max_bytes(),
            1024,
            "re-exported limit should preserve bounded requests"
        );
    }
}
