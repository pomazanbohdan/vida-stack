#![allow(unexpected_cfgs)]

/// Bounded proof entrypoints. Kani supplies `kani` and enables `cfg(kani)`;
/// normal Cargo builds intentionally contain no proof execution.
#[cfg(kani)]
mod proofs {
    use kani::any;
    use taskflow_core::role_step::TaskRoleStep;
    use taskflow_core::run_workflow::{RunWorkflowAggregate, RunWorkflowCommand};

    #[kani::proof]
    fn bounded_run_workflow_transition_preserves_version_bound() {
        let initial_version: u64 = any();
        kani::assume(initial_version < 1024);
        let mut aggregate = RunWorkflowAggregate::from_snapshot(
            "kani-run",
            "kani-task",
            taskflow_core::run_workflow::RunWorkflowState::Idle,
            initial_version,
        );
        let _ = aggregate.handle(RunWorkflowCommand::Start {
            first_step: TaskRoleStep::planning(),
        });
        assert!(aggregate.version <= initial_version + 1);
    }
}
