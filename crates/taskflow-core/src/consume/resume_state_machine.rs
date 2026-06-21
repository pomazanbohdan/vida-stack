//! Isolated consume/resume lifecycle pilot backed by a declarative FSM.

use rust_fsm::state_machine;

state_machine! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub resume_lifecycle(Idle)

    Idle(DispatchStarted) => Dispatching,
    Dispatching(ResumeSucceeded) => Resumed,
    Dispatching(ResumeBlocked) => Blocked,
    Blocked(DispatchStarted) => Dispatching,
}

pub use resume_lifecycle::{Input as ResumeLifecycleEvent, State as ResumeLifecycleState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResumeLifecycleTransitionError {
    pub from: ResumeLifecycleState,
    pub event: ResumeLifecycleEvent,
}

impl std::fmt::Display for ResumeLifecycleTransitionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "resume lifecycle transition from {:?} with {:?} is not allowed",
            self.from, self.event
        )
    }
}

impl std::error::Error for ResumeLifecycleTransitionError {}

#[must_use]
pub fn initial_resume_lifecycle_state() -> ResumeLifecycleState {
    *resume_lifecycle::StateMachine::new().state()
}

pub fn advance_resume_lifecycle(
    from: ResumeLifecycleState,
    event: ResumeLifecycleEvent,
) -> Result<ResumeLifecycleState, ResumeLifecycleTransitionError> {
    let mut machine = resume_lifecycle::StateMachine::from_state(from);
    machine
        .consume(&event)
        .map_err(|_| ResumeLifecycleTransitionError { from, event })?;
    Ok(*machine.state())
}

#[cfg(test)]
mod tests {
    use super::{
        ResumeLifecycleEvent, ResumeLifecycleState, advance_resume_lifecycle,
        initial_resume_lifecycle_state,
    };

    #[test]
    fn resume_lifecycle_starts_idle() {
        assert_eq!(initial_resume_lifecycle_state(), ResumeLifecycleState::Idle);
    }

    #[test]
    fn resume_lifecycle_allows_dispatch_success_path() {
        let dispatching = advance_resume_lifecycle(
            ResumeLifecycleState::Idle,
            ResumeLifecycleEvent::DispatchStarted,
        )
        .expect("idle should move to dispatching");

        assert_eq!(dispatching, ResumeLifecycleState::Dispatching);
        assert_eq!(
            advance_resume_lifecycle(dispatching, ResumeLifecycleEvent::ResumeSucceeded)
                .expect("dispatching should move to resumed"),
            ResumeLifecycleState::Resumed
        );
    }

    #[test]
    fn resume_lifecycle_allows_blocked_retry_path() {
        let blocked = advance_resume_lifecycle(
            ResumeLifecycleState::Dispatching,
            ResumeLifecycleEvent::ResumeBlocked,
        )
        .expect("dispatching should move to blocked");

        assert_eq!(blocked, ResumeLifecycleState::Blocked);
        assert_eq!(
            advance_resume_lifecycle(blocked, ResumeLifecycleEvent::DispatchStarted)
                .expect("blocked should retry through dispatching"),
            ResumeLifecycleState::Dispatching
        );
    }

    #[test]
    fn resume_lifecycle_rejects_invalid_transitions() {
        let error = advance_resume_lifecycle(
            ResumeLifecycleState::Idle,
            ResumeLifecycleEvent::ResumeSucceeded,
        )
        .expect_err("idle cannot skip dispatch");

        assert_eq!(error.from, ResumeLifecycleState::Idle);
        assert_eq!(error.event, ResumeLifecycleEvent::ResumeSucceeded);
    }
}
