use crate::state_store::{RunGraphStatus, StateStore, StateStoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunGraphTaskAuthorityKind {
    AuthoritativeTaskPresent,
    BlockedTaskAdmissionDenied,
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

    pub(crate) fn task_blocked(&self) -> bool {
        self.kind == RunGraphTaskAuthorityKind::BlockedTaskAdmissionDenied
    }

    pub(crate) fn continuation_admission_denied(&self) -> bool {
        matches!(
            self.kind,
            RunGraphTaskAuthorityKind::BlockedTaskAdmissionDenied
                | RunGraphTaskAuthorityKind::ClosedTaskStaleRun
                | RunGraphTaskAuthorityKind::MissingTaskStaleRun
        )
    }

    pub(crate) fn task_closed(&self) -> bool {
        self.task_status
            .as_deref()
            .is_some_and(StateStore::task_status_is_closed_like)
            || matches!(
                self.kind,
                RunGraphTaskAuthorityKind::ClosedTaskStaleRun
                    | RunGraphTaskAuthorityKind::TerminalClosureOk
            )
    }

    pub(crate) fn stale_for_active_projection(&self) -> bool {
        matches!(
            self.kind,
            RunGraphTaskAuthorityKind::BlockedTaskAdmissionDenied
                | RunGraphTaskAuthorityKind::ClosedTaskStaleRun
                | RunGraphTaskAuthorityKind::MissingTaskStaleRun
        )
    }
}

pub(crate) fn run_graph_status_is_terminal_closure(status: &RunGraphStatus) -> bool {
    status.is_terminal_closure()
}

pub(crate) fn terminal_task_active_status_matches_current_run(
    latest_status: Option<&RunGraphStatus>,
    terminal_status: &RunGraphStatus,
) -> bool {
    latest_status.is_some_and(|current| current.run_id == terminal_status.run_id)
}

fn identity_authority_candidate_ids(
    identity: Option<&crate::state_store::RunGraphDispatchTaskIdentity>,
) -> Vec<String> {
    let mut visited = std::collections::BTreeSet::new();
    identity
        .into_iter()
        .flat_map(|identity| {
            [
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
        Ok(task)
            if task.id == status.task_id
                && matches!(
                    taskflow_core::parse_task_status(&task.status),
                    Some(taskflow_core::TaskStatus::Blocked)
                ) =>
        {
            Ok(Some(RunGraphTaskAuthorityVerdict {
                kind: RunGraphTaskAuthorityKind::BlockedTaskAdmissionDenied,
                run_id: status.run_id.clone(),
                task_id: task.id,
                task_status: Some(task.status),
            }))
        }
        Ok(task) if !StateStore::task_status_is_closed_like(&task.status) => {
            Ok(Some(RunGraphTaskAuthorityVerdict {
                kind: RunGraphTaskAuthorityKind::AuthoritativeTaskPresent,
                run_id: status.run_id.clone(),
                task_id: task.id,
                task_status: Some(task.status),
            }))
        }
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
        Ok(task)
            if matches!(
                taskflow_core::parse_task_status(&task.status),
                Some(taskflow_core::TaskStatus::Blocked)
            ) =>
        {
            Ok(RunGraphTaskAuthorityVerdict {
                kind: RunGraphTaskAuthorityKind::BlockedTaskAdmissionDenied,
                run_id: status.run_id.clone(),
                task_id: status.task_id.clone(),
                task_status: Some(task.status),
            })
        }
        Ok(task) if StateStore::task_status_is_closed_like(&task.status) => {
            Ok(RunGraphTaskAuthorityVerdict {
                kind: RunGraphTaskAuthorityKind::ClosedTaskStaleRun,
                run_id: status.run_id.clone(),
                task_id: status.task_id.clone(),
                task_status: Some(task.status),
            })
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_store::{TaskExecutionSemantics, TaskPlannerMetadata, TaskRecord};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_authority_root(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }

    fn test_task_record(task_id: &str, status: &str) -> TaskRecord {
        TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: task_id.to_string(),
            description: task_id.to_string(),
            status: status.to_string(),
            priority: 1,
            issue_type: "task".to_string(),
            created_at: "2026-06-18T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-06-18T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: TaskExecutionSemantics::default(),
            planner_metadata: TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn authority_verdict_task_closed_uses_canonical_aliases() {
        for alias in ["closed", "completed", "done", "resolved", "merged"] {
            let verdict = RunGraphTaskAuthorityVerdict {
                kind: RunGraphTaskAuthorityKind::AuthoritativeTaskPresent,
                run_id: "run-alias".to_string(),
                task_id: "task-alias".to_string(),
                task_status: Some(alias.to_string()),
            };
            assert!(verdict.task_closed(), "{alias} should be closed-like");
        }
    }

    #[tokio::test]
    async fn authority_verdict_denies_blocked_task_admission_for_matching_run() {
        let root = temp_authority_root("vida-run-graph-authority-blocked-task");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record("blocked-task", "blocked"))
            .await
            .expect("persist blocked task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "blocked-run",
            "implementation",
            "implementation",
        );
        status.task_id = "blocked-task".to_string();
        status.status = "blocked".to_string();

        let verdict = run_graph_task_authority_verdict(&store, &status)
            .await
            .expect("authority verdict");
        assert_eq!(
            verdict.kind,
            RunGraphTaskAuthorityKind::BlockedTaskAdmissionDenied
        );
        assert_eq!(verdict.run_id, "blocked-run");
        assert_eq!(verdict.task_id, "blocked-task");
        assert_eq!(verdict.task_status.as_deref(), Some("blocked"));
        assert!(verdict.task_blocked());
        assert!(verdict.continuation_admission_denied());
        assert!(verdict.stale_for_active_projection());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn authority_verdict_keeps_unrelated_session_admissible() {
        let root = temp_authority_root("vida-run-graph-authority-unrelated-session");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record("blocked-task", "blocked"))
            .await
            .expect("persist blocked task");
        store
            .persist_task_record(test_task_record("unrelated-task", "open"))
            .await
            .expect("persist unrelated task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "unrelated-run",
            "implementation",
            "implementation",
        );
        status.task_id = "unrelated-task".to_string();
        status.status = "in_progress".to_string();

        let verdict = run_graph_task_authority_verdict(&store, &status)
            .await
            .expect("authority verdict");
        assert_eq!(
            verdict.kind,
            RunGraphTaskAuthorityKind::AuthoritativeTaskPresent
        );
        assert!(!verdict.task_blocked());
        assert!(!verdict.continuation_admission_denied());
        assert!(!verdict.stale_for_active_projection());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn terminal_task_active_status_requires_matching_current_run() {
        let current = crate::taskflow_run_graph::default_run_graph_status(
            "current-run",
            "implementation",
            "implementation",
        );
        let mut terminal = crate::taskflow_run_graph::default_run_graph_status(
            "current-run",
            "implementation",
            "implementation",
        );
        terminal.active_node = "closure".to_string();
        terminal.status = "completed".to_string();
        terminal.lifecycle_stage = "closure_complete".to_string();

        assert!(terminal_task_active_status_matches_current_run(
            Some(&current),
            &terminal
        ));
        assert!(!terminal_task_active_status_matches_current_run(
            None, &terminal
        ));

        let other = crate::taskflow_run_graph::default_run_graph_status(
            "other-run",
            "implementation",
            "implementation",
        );
        assert!(!terminal_task_active_status_matches_current_run(
            Some(&other),
            &terminal
        ));
    }

    #[tokio::test]
    async fn authority_verdict_treats_closed_alias_task_as_stale_run() {
        let root = temp_authority_root("vida-run-graph-authority-status-alias");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record("alias-task", "resolved"))
            .await
            .expect("persist task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-alias",
            "implementation",
            "implementation",
        );
        status.task_id = "alias-task".to_string();
        status.status = "blocked".to_string();

        let verdict = run_graph_task_authority_verdict(&store, &status)
            .await
            .expect("authority verdict");
        assert_eq!(verdict.kind, RunGraphTaskAuthorityKind::ClosedTaskStaleRun);
        assert_eq!(verdict.task_status.as_deref(), Some("resolved"));
        assert!(verdict.task_closed());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn authority_verdict_does_not_let_feature_parent_mask_missing_active_task() {
        let root = temp_authority_root("vida-run-graph-authority-feature-parent-mask");
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .persist_task_record(test_task_record("feature-parent", "open"))
            .await
            .expect("persist feature parent");
        store
            .record_run_graph_dispatch_task_identity(
                &crate::state_store::RunGraphDispatchTaskIdentity {
                    run_id: "missing-dev-task".to_string(),
                    feature_epic_id: Some("feature-parent".to_string()),
                    spec_task_id: None,
                    work_pool_task_id: None,
                    dev_task_id: Some("missing-dev-task".to_string()),
                    source: "dispatch_init_existing_task".to_string(),
                    updated_at: "2026-07-02T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("record task identity");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "missing-dev-task",
            "implementation",
            "implementation",
        );
        status.task_id = "missing-dev-task".to_string();
        status.status = "blocked".to_string();

        let verdict = run_graph_task_authority_verdict(&store, &status)
            .await
            .expect("authority verdict");
        assert_eq!(verdict.kind, RunGraphTaskAuthorityKind::MissingTaskStaleRun);
        assert_eq!(verdict.task_status, None);

        let _ = std::fs::remove_dir_all(&root);
    }
}
