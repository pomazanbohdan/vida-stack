use crate::state_store::{RunGraphStatus, StateStore, StateStoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunGraphTaskAuthorityKind {
    AuthoritativeTaskPresent,
    ClosedTaskStaleRun,
    MissingTaskStaleRun,
    TerminalClosureOk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunGraphTaskAuthorityVerdict {
    pub(crate) kind: RunGraphTaskAuthorityKind,
    pub(crate) run_id: String,
    pub(crate) task_id: String,
    pub(crate) task_status: Option<String>,
}

impl RunGraphTaskAuthorityVerdict {
    pub(crate) fn task_missing(&self) -> bool {
        self.kind == RunGraphTaskAuthorityKind::MissingTaskStaleRun
    }

    pub(crate) fn task_closed_stale_run(&self) -> bool {
        self.kind == RunGraphTaskAuthorityKind::ClosedTaskStaleRun
    }

    pub(crate) fn task_closed(&self) -> bool {
        self.task_status.as_deref() == Some("closed")
            || matches!(
                self.kind,
                RunGraphTaskAuthorityKind::ClosedTaskStaleRun
                    | RunGraphTaskAuthorityKind::TerminalClosureOk
            )
    }

    pub(crate) fn stale_for_active_projection(&self) -> bool {
        matches!(
            self.kind,
            RunGraphTaskAuthorityKind::ClosedTaskStaleRun
                | RunGraphTaskAuthorityKind::MissingTaskStaleRun
        )
    }
}

pub(crate) fn run_graph_status_is_terminal_closure(status: &RunGraphStatus) -> bool {
    status.status == "completed"
        && status.lifecycle_stage == "closure_complete"
        && status
            .next_node
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        && status.resume_target == "none"
}

fn identity_authority_candidate_ids(
    identity: Option<&crate::state_store::RunGraphDispatchTaskIdentity>,
) -> Vec<String> {
    let mut visited = std::collections::BTreeSet::new();
    identity
        .into_iter()
        .flat_map(|identity| {
            [
                identity.feature_epic_id.as_deref(),
                identity.dev_task_id.as_deref(),
                identity.work_pool_task_id.as_deref(),
                identity.spec_task_id.as_deref(),
            ]
        })
        .flatten()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| visited.insert((*value).to_string()))
        .map(str::to_string)
        .collect()
}

async fn authoritative_present_task(
    store: &StateStore,
    status: &RunGraphStatus,
    task_id: &str,
) -> Result<Option<RunGraphTaskAuthorityVerdict>, StateStoreError> {
    match store.show_task(task_id).await {
        Ok(task) if task.status != "closed" => Ok(Some(RunGraphTaskAuthorityVerdict {
            kind: RunGraphTaskAuthorityKind::AuthoritativeTaskPresent,
            run_id: status.run_id.clone(),
            task_id: task.id,
            task_status: Some(task.status),
        })),
        Ok(_) | Err(StateStoreError::MissingTask { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) async fn run_graph_task_authority_verdict(
    store: &StateStore,
    status: &RunGraphStatus,
) -> Result<RunGraphTaskAuthorityVerdict, StateStoreError> {
    if run_graph_status_is_terminal_closure(status)
        && store
            .run_graph_terminal_closure_has_task_close_truth(status)
            .await?
    {
        return Ok(RunGraphTaskAuthorityVerdict {
            kind: RunGraphTaskAuthorityKind::TerminalClosureOk,
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            task_status: Some("closed".to_string()),
        });
    }

    let identity = store
        .run_graph_dispatch_task_identity(&status.run_id)
        .await?;
    for candidate_id in identity_authority_candidate_ids(identity.as_ref()) {
        if let Some(verdict) = authoritative_present_task(store, status, &candidate_id).await? {
            return Ok(verdict);
        }
    }

    match store.show_task(&status.task_id).await {
        Ok(task) if task.status == "closed" => Ok(RunGraphTaskAuthorityVerdict {
            kind: RunGraphTaskAuthorityKind::ClosedTaskStaleRun,
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            task_status: Some(task.status),
        }),
        Ok(task) => Ok(RunGraphTaskAuthorityVerdict {
            kind: RunGraphTaskAuthorityKind::AuthoritativeTaskPresent,
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            task_status: Some(task.status),
        }),
        Err(StateStoreError::MissingTask { .. }) => Ok(RunGraphTaskAuthorityVerdict {
            kind: RunGraphTaskAuthorityKind::MissingTaskStaleRun,
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            task_status: None,
        }),
        Err(error) => Err(error),
    }
}
