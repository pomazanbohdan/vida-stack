pub mod authority_chain;
pub mod continuation_binding;
pub mod errors;
pub mod exception_takeover;
pub mod final_snapshot;
pub mod projection_cache;
pub mod run_graph_transition;
pub mod stale_guard;
pub mod task_transition;
pub mod terminal_closure;

pub use errors::TaskflowAuthorityError;

#[cfg(test)]
mod tests {
    #[test]
    fn public_authority_modules_are_registered() {
        let modules = [
            crate::authority_chain::MODULE,
            crate::terminal_closure::MODULE,
            crate::stale_guard::MODULE,
            crate::exception_takeover::MODULE,
            crate::projection_cache::MODULE,
            crate::run_graph_transition::MODULE,
            crate::final_snapshot::MODULE,
            crate::continuation_binding::MODULE,
            crate::task_transition::MODULE,
        ];

        assert_eq!(modules.len(), 9);
        assert!(modules.contains(&"authority_chain"));
        assert!(modules.contains(&"terminal_closure"));
        assert!(modules.contains(&"stale_guard"));
        assert!(modules.contains(&"exception_takeover"));
        assert!(modules.contains(&"projection_cache"));
        assert!(modules.contains(&"run_graph_transition"));
        assert!(modules.contains(&"final_snapshot"));
        assert!(modules.contains(&"continuation_binding"));
        assert!(modules.contains(&"task_transition"));
    }
}
