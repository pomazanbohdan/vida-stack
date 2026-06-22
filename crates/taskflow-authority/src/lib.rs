pub mod authority_chain;
pub mod continuation_binding;
pub mod continuation_transition;
pub mod errors;
pub mod exception_takeover;
pub mod final_snapshot;
pub mod operation_authorization;
pub mod projection_cache;
pub mod role_step;
pub mod run_graph_evidence;
pub mod run_graph_transition;
pub mod run_workflow;
pub mod scheduler_claim;
pub mod stale_guard;
pub mod task_attempts;
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
            crate::run_graph_evidence::MODULE,
            crate::run_graph_transition::MODULE,
            crate::run_workflow::MODULE,
            crate::role_step::MODULE,
            crate::scheduler_claim::MODULE,
            crate::final_snapshot::MODULE,
            crate::operation_authorization::MODULE,
            crate::continuation_binding::MODULE,
            crate::continuation_transition::MODULE,
            crate::task_attempts::MODULE,
            crate::task_transition::MODULE,
        ];

        assert_eq!(modules.len(), 16);
        assert!(modules.contains(&"authority_chain"));
        assert!(modules.contains(&"terminal_closure"));
        assert!(modules.contains(&"stale_guard"));
        assert!(modules.contains(&"exception_takeover"));
        assert!(modules.contains(&"projection_cache"));
        assert!(modules.contains(&"run_graph_evidence"));
        assert!(modules.contains(&"run_graph_transition"));
        assert!(modules.contains(&"run_workflow"));
        assert!(modules.contains(&"role_step"));
        assert!(modules.contains(&"scheduler_claim"));
        assert!(modules.contains(&"final_snapshot"));
        assert!(modules.contains(&"operation_authorization"));
        assert!(modules.contains(&"continuation_binding"));
        assert!(modules.contains(&"continuation_transition"));
        assert!(modules.contains(&"task_attempts"));
        assert!(modules.contains(&"task_transition"));
    }
}
