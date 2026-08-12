use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAggregateTaskSnapshot {
    pub id: String,
    pub status: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub close_reason: Option<String>,
    pub parent_id: Option<String>,
}

impl TaskAggregateTaskSnapshot {
    #[must_use]
    pub fn closed(
        id: impl Into<String>,
        updated_at: impl Into<String>,
        parent_id: Option<String>,
    ) -> Self {
        let updated_at = updated_at.into();
        Self {
            id: id.into(),
            status: "closed".to_string(),
            updated_at: updated_at.clone(),
            closed_at: Some(updated_at),
            close_reason: None,
            parent_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCloseCommand {
    pub task: TaskAggregateTaskSnapshot,
    pub reason: String,
    pub occurred_at: String,
    pub auto_closed_parents: Vec<TaskAggregateTaskSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStatusUpdateCommand {
    pub task: TaskAggregateTaskSnapshot,
    pub occurred_at: String,
    pub auto_closed_parents: Vec<TaskAggregateTaskSnapshot>,
    pub auto_reopened_parents: Vec<TaskAggregateTaskSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCreateCommand {
    pub task: TaskAggregateTaskSnapshot,
    pub occurred_at: String,
    pub auto_reopened_parents: Vec<TaskAggregateTaskSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskReparentCommand {
    pub moved_tasks: Vec<TaskAggregateTaskSnapshot>,
    pub from_parent_id: String,
    pub to_parent_id: String,
    pub occurred_at: String,
    pub auto_closed_parents: Vec<TaskAggregateTaskSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDependencyMutationCommand {
    pub task_id: String,
    pub depends_on_id: String,
    pub edge_type: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMetadataUpdateCommand {
    pub task_id: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TaskAggregateEvent {
    TaskClosed {
        task_id: String,
        reason: String,
        occurred_at: String,
    },
    ParentAutoClosed {
        task_id: String,
        reason: String,
        occurred_at: String,
        source_child_id: String,
    },
    TaskStatusUpdated {
        task_id: String,
        status: String,
        occurred_at: String,
    },
    ParentAutoReopened {
        task_id: String,
        occurred_at: String,
        source_child_id: String,
    },
    TaskCreated {
        task_id: String,
        status: String,
        parent_id: Option<String>,
        occurred_at: String,
    },
    TaskReparented {
        task_id: String,
        from_parent_id: String,
        to_parent_id: String,
        occurred_at: String,
    },
    TaskDependencyAdded {
        task_id: String,
        depends_on_id: String,
        edge_type: String,
        occurred_at: String,
    },
    TaskDependencyRemoved {
        task_id: String,
        depends_on_id: String,
        edge_type: String,
        occurred_at: String,
    },
    TaskMetadataUpdated {
        task_id: String,
        occurred_at: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskAggregateMutation {
    CloseTask {
        task_id: String,
        status: String,
        updated_at: String,
        closed_at: String,
        close_reason: String,
    },
    AutoCloseParent {
        task_id: String,
        status: String,
        updated_at: String,
        closed_at: String,
        close_reason: String,
    },
    SetTaskStatus {
        task_id: String,
        status: String,
        updated_at: String,
        closed_at: Option<String>,
        close_reason: Option<String>,
    },
    AutoReopenParent {
        task_id: String,
        status: String,
        updated_at: String,
    },
    CreateTask {
        task_id: String,
        status: String,
        updated_at: String,
        closed_at: Option<String>,
    },
    ReparentTask {
        task_id: String,
        parent_id: Option<String>,
        updated_at: String,
    },
    AddDependency {
        task_id: String,
        depends_on_id: String,
        edge_type: String,
        updated_at: String,
    },
    RemoveDependency {
        task_id: String,
        depends_on_id: String,
        edge_type: String,
        updated_at: String,
    },
    UpdateTaskMetadata {
        task_id: String,
        updated_at: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMutationPlan {
    pub events: Vec<TaskAggregateEvent>,
    pub mutations: Vec<TaskAggregateMutation>,
    pub touched_task_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMutationPlanCoverageError {
    pub blocker_code: &'static str,
    pub expected_task_ids: Vec<String>,
    pub actual_task_ids: Vec<String>,
}

pub const TASK_AGGREGATE_PLAN_BLOCKER_EMPTY_EVENTS: &str = "task_aggregate_plan_empty_events";
pub const TASK_AGGREGATE_PLAN_BLOCKER_EMPTY_MUTATIONS: &str = "task_aggregate_plan_empty_mutations";
pub const TASK_AGGREGATE_PLAN_BLOCKER_TOUCH_MISMATCH: &str = "task_aggregate_plan_touch_mismatch";

#[must_use]
pub fn task_mutation_plan_persisted_task_ids(plan: &TaskMutationPlan) -> BTreeSet<String> {
    plan.touched_task_ids
        .iter()
        .map(|task_id| task_id.trim())
        .filter(|task_id| !task_id.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn ensure_task_mutation_plan_covers_persistence(
    plan: &TaskMutationPlan,
    persisted_task_ids: &BTreeSet<String>,
) -> Result<(), TaskMutationPlanCoverageError> {
    if plan.events.is_empty() {
        return Err(TaskMutationPlanCoverageError {
            blocker_code: TASK_AGGREGATE_PLAN_BLOCKER_EMPTY_EVENTS,
            expected_task_ids: persisted_task_ids.iter().cloned().collect(),
            actual_task_ids: Vec::new(),
        });
    }
    if plan.mutations.is_empty() {
        return Err(TaskMutationPlanCoverageError {
            blocker_code: TASK_AGGREGATE_PLAN_BLOCKER_EMPTY_MUTATIONS,
            expected_task_ids: persisted_task_ids.iter().cloned().collect(),
            actual_task_ids: Vec::new(),
        });
    }

    let actual_task_ids = task_mutation_plan_persisted_task_ids(plan);
    if &actual_task_ids != persisted_task_ids {
        return Err(TaskMutationPlanCoverageError {
            blocker_code: TASK_AGGREGATE_PLAN_BLOCKER_TOUCH_MISMATCH,
            expected_task_ids: persisted_task_ids.iter().cloned().collect(),
            actual_task_ids: actual_task_ids.into_iter().collect(),
        });
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskAggregateProjection {
    pub statuses: BTreeMap<String, String>,
    pub parent_ids: BTreeMap<String, Option<String>>,
    pub dependencies: BTreeMap<String, Vec<TaskAggregateDependencyEdge>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskAggregateDependencyEdge {
    pub depends_on_id: String,
    pub edge_type: String,
}

#[must_use]
pub fn plan_close_task(command: TaskCloseCommand) -> TaskMutationPlan {
    let task_id = command.task.id;
    let mut events = vec![TaskAggregateEvent::TaskClosed {
        task_id: task_id.clone(),
        reason: command.reason.clone(),
        occurred_at: command.occurred_at.clone(),
    }];
    let mut mutations = vec![TaskAggregateMutation::CloseTask {
        task_id: task_id.clone(),
        status: "closed".to_string(),
        updated_at: command.occurred_at.clone(),
        closed_at: command.occurred_at.clone(),
        close_reason: command.reason,
    }];
    let mut touched_task_ids = vec![task_id.clone()];

    for parent in command.auto_closed_parents {
        let parent_reason = parent
            .close_reason
            .unwrap_or_else(|| format!("all direct child tasks closed after closing `{task_id}`"));
        events.push(TaskAggregateEvent::ParentAutoClosed {
            task_id: parent.id.clone(),
            reason: parent_reason.clone(),
            occurred_at: parent.updated_at.clone(),
            source_child_id: task_id.clone(),
        });
        mutations.push(TaskAggregateMutation::AutoCloseParent {
            task_id: parent.id.clone(),
            status: "closed".to_string(),
            updated_at: parent.updated_at.clone(),
            closed_at: parent.closed_at.unwrap_or(parent.updated_at),
            close_reason: parent_reason,
        });
        touched_task_ids.push(parent.id);
    }

    TaskMutationPlan {
        events,
        mutations,
        touched_task_ids,
    }
}

#[must_use]
pub fn plan_update_task_status(command: TaskStatusUpdateCommand) -> TaskMutationPlan {
    let task_id = command.task.id;
    let mut events = vec![TaskAggregateEvent::TaskStatusUpdated {
        task_id: task_id.clone(),
        status: command.task.status.clone(),
        occurred_at: command.occurred_at.clone(),
    }];
    let mut mutations = vec![TaskAggregateMutation::SetTaskStatus {
        task_id: task_id.clone(),
        status: command.task.status,
        updated_at: command.occurred_at,
        closed_at: command.task.closed_at,
        close_reason: command.task.close_reason,
    }];
    let mut touched_task_ids = vec![task_id.clone()];

    for parent in command.auto_closed_parents {
        let parent_reason = parent
            .close_reason
            .unwrap_or_else(|| format!("all direct child tasks closed after closing `{task_id}`"));
        events.push(TaskAggregateEvent::ParentAutoClosed {
            task_id: parent.id.clone(),
            reason: parent_reason.clone(),
            occurred_at: parent.updated_at.clone(),
            source_child_id: task_id.clone(),
        });
        mutations.push(TaskAggregateMutation::AutoCloseParent {
            task_id: parent.id.clone(),
            status: "closed".to_string(),
            updated_at: parent.updated_at.clone(),
            closed_at: parent.closed_at.unwrap_or(parent.updated_at),
            close_reason: parent_reason,
        });
        touched_task_ids.push(parent.id);
    }

    for parent in command.auto_reopened_parents {
        events.push(TaskAggregateEvent::ParentAutoReopened {
            task_id: parent.id.clone(),
            occurred_at: parent.updated_at.clone(),
            source_child_id: task_id.clone(),
        });
        mutations.push(TaskAggregateMutation::AutoReopenParent {
            task_id: parent.id.clone(),
            status: parent.status,
            updated_at: parent.updated_at,
        });
        touched_task_ids.push(parent.id);
    }

    TaskMutationPlan {
        events,
        mutations,
        touched_task_ids,
    }
}

#[must_use]
pub fn plan_create_task(command: TaskCreateCommand) -> TaskMutationPlan {
    let task_id = command.task.id;
    let mut events = vec![TaskAggregateEvent::TaskCreated {
        task_id: task_id.clone(),
        status: command.task.status.clone(),
        parent_id: command.task.parent_id.clone(),
        occurred_at: command.occurred_at.clone(),
    }];
    let mut mutations = vec![TaskAggregateMutation::CreateTask {
        task_id: task_id.clone(),
        status: command.task.status,
        updated_at: command.occurred_at,
        closed_at: command.task.closed_at,
    }];
    let mut touched_task_ids = vec![task_id.clone()];

    for parent in command.auto_reopened_parents {
        events.push(TaskAggregateEvent::ParentAutoReopened {
            task_id: parent.id.clone(),
            occurred_at: parent.updated_at.clone(),
            source_child_id: task_id.clone(),
        });
        mutations.push(TaskAggregateMutation::AutoReopenParent {
            task_id: parent.id.clone(),
            status: parent.status,
            updated_at: parent.updated_at,
        });
        touched_task_ids.push(parent.id);
    }

    TaskMutationPlan {
        events,
        mutations,
        touched_task_ids,
    }
}

#[must_use]
pub fn replay_task_events(events: &[TaskAggregateEvent]) -> TaskAggregateProjection {
    let mut projection = TaskAggregateProjection::default();
    for event in events {
        match event {
            TaskAggregateEvent::TaskClosed { task_id, .. }
            | TaskAggregateEvent::ParentAutoClosed { task_id, .. } => {
                projection
                    .statuses
                    .insert(task_id.clone(), "closed".to_string());
            }
            TaskAggregateEvent::TaskStatusUpdated {
                task_id, status, ..
            } => {
                projection.statuses.insert(task_id.clone(), status.clone());
            }
            TaskAggregateEvent::ParentAutoReopened { task_id, .. } => {
                projection
                    .statuses
                    .insert(task_id.clone(), "in_progress".to_string());
            }
            TaskAggregateEvent::TaskCreated {
                task_id,
                status,
                parent_id,
                ..
            } => {
                projection.statuses.insert(task_id.clone(), status.clone());
                projection
                    .parent_ids
                    .insert(task_id.clone(), parent_id.clone());
            }
            TaskAggregateEvent::TaskReparented {
                task_id,
                to_parent_id,
                ..
            } => {
                projection
                    .parent_ids
                    .insert(task_id.clone(), Some(to_parent_id.clone()));
            }
            TaskAggregateEvent::TaskDependencyAdded {
                task_id,
                depends_on_id,
                edge_type,
                ..
            } => {
                let edges = projection.dependencies.entry(task_id.clone()).or_default();
                let edge = TaskAggregateDependencyEdge {
                    depends_on_id: depends_on_id.clone(),
                    edge_type: edge_type.clone(),
                };
                if !edges.contains(&edge) {
                    edges.push(edge);
                    edges.sort();
                }
            }
            TaskAggregateEvent::TaskDependencyRemoved {
                task_id,
                depends_on_id,
                edge_type,
                ..
            } => {
                if let Some(edges) = projection.dependencies.get_mut(task_id) {
                    edges.retain(|edge| {
                        edge.depends_on_id != *depends_on_id || edge.edge_type != *edge_type
                    });
                }
            }
            TaskAggregateEvent::TaskMetadataUpdated { .. } => {}
        }
    }
    projection
}

#[must_use]
pub fn plan_update_task_metadata(command: TaskMetadataUpdateCommand) -> TaskMutationPlan {
    TaskMutationPlan {
        events: vec![TaskAggregateEvent::TaskMetadataUpdated {
            task_id: command.task_id.clone(),
            occurred_at: command.occurred_at.clone(),
        }],
        mutations: vec![TaskAggregateMutation::UpdateTaskMetadata {
            task_id: command.task_id.clone(),
            updated_at: command.occurred_at,
        }],
        touched_task_ids: vec![command.task_id],
    }
}

#[must_use]
pub fn plan_add_task_dependency(command: TaskDependencyMutationCommand) -> TaskMutationPlan {
    let touched_task_ids = dependency_touched_task_ids(&command.task_id, &command.depends_on_id);
    TaskMutationPlan {
        events: vec![TaskAggregateEvent::TaskDependencyAdded {
            task_id: command.task_id.clone(),
            depends_on_id: command.depends_on_id.clone(),
            edge_type: command.edge_type.clone(),
            occurred_at: command.occurred_at.clone(),
        }],
        mutations: vec![TaskAggregateMutation::AddDependency {
            task_id: command.task_id,
            depends_on_id: command.depends_on_id,
            edge_type: command.edge_type,
            updated_at: command.occurred_at,
        }],
        touched_task_ids,
    }
}

#[must_use]
pub fn plan_remove_task_dependency(command: TaskDependencyMutationCommand) -> TaskMutationPlan {
    let touched_task_ids = dependency_touched_task_ids(&command.task_id, &command.depends_on_id);
    TaskMutationPlan {
        events: vec![TaskAggregateEvent::TaskDependencyRemoved {
            task_id: command.task_id.clone(),
            depends_on_id: command.depends_on_id.clone(),
            edge_type: command.edge_type.clone(),
            occurred_at: command.occurred_at.clone(),
        }],
        mutations: vec![TaskAggregateMutation::RemoveDependency {
            task_id: command.task_id,
            depends_on_id: command.depends_on_id,
            edge_type: command.edge_type,
            updated_at: command.occurred_at,
        }],
        touched_task_ids,
    }
}

fn dependency_touched_task_ids(task_id: &str, depends_on_id: &str) -> Vec<String> {
    let mut touched_task_ids = vec![task_id.to_string(), depends_on_id.to_string()];
    touched_task_ids.sort();
    touched_task_ids.dedup();
    touched_task_ids
}

#[must_use]
pub fn plan_reparent_tasks(command: TaskReparentCommand) -> TaskMutationPlan {
    let mut events = Vec::new();
    let mut mutations = Vec::new();
    let mut touched_task_ids = Vec::new();

    for task in command.moved_tasks {
        events.push(TaskAggregateEvent::TaskReparented {
            task_id: task.id.clone(),
            from_parent_id: command.from_parent_id.clone(),
            to_parent_id: command.to_parent_id.clone(),
            occurred_at: task.updated_at.clone(),
        });
        mutations.push(TaskAggregateMutation::ReparentTask {
            task_id: task.id.clone(),
            parent_id: task.parent_id,
            updated_at: task.updated_at,
        });
        touched_task_ids.push(task.id);
    }
    touched_task_ids.push(command.from_parent_id.clone());
    touched_task_ids.push(command.to_parent_id);

    for parent in command.auto_closed_parents {
        let parent_reason = parent.close_reason.unwrap_or_else(|| {
            format!(
                "all direct child tasks moved from `{}`",
                command.from_parent_id
            )
        });
        events.push(TaskAggregateEvent::ParentAutoClosed {
            task_id: parent.id.clone(),
            reason: parent_reason.clone(),
            occurred_at: parent.updated_at.clone(),
            source_child_id: command.from_parent_id.clone(),
        });
        mutations.push(TaskAggregateMutation::AutoCloseParent {
            task_id: parent.id.clone(),
            status: "closed".to_string(),
            updated_at: parent.updated_at.clone(),
            closed_at: parent.closed_at.unwrap_or(parent.updated_at),
            close_reason: parent_reason,
        });
        touched_task_ids.push(parent.id);
    }

    touched_task_ids.sort();
    touched_task_ids.dedup();

    TaskMutationPlan {
        events,
        mutations,
        touched_task_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_aggregate_plans_close_task_event_and_mutation() {
        let plan = plan_close_task(TaskCloseCommand {
            task: TaskAggregateTaskSnapshot::closed("child", "100", Some("parent".to_string())),
            reason: "proof passed".to_string(),
            occurred_at: "100".to_string(),
            auto_closed_parents: Vec::new(),
        });

        assert_eq!(
            plan.events,
            vec![TaskAggregateEvent::TaskClosed {
                task_id: "child".to_string(),
                reason: "proof passed".to_string(),
                occurred_at: "100".to_string(),
            }]
        );
        assert_eq!(plan.touched_task_ids, vec!["child"]);
    }

    #[test]
    fn replay_task_events_deduplicates_and_removes_dependency_edges() {
        let events = vec![
            TaskAggregateEvent::TaskDependencyAdded {
                task_id: "task".to_string(),
                depends_on_id: "dep-b".to_string(),
                edge_type: "blocks".to_string(),
                occurred_at: "1".to_string(),
            },
            TaskAggregateEvent::TaskDependencyAdded {
                task_id: "task".to_string(),
                depends_on_id: "dep-a".to_string(),
                edge_type: "parent-child".to_string(),
                occurred_at: "2".to_string(),
            },
            TaskAggregateEvent::TaskDependencyAdded {
                task_id: "task".to_string(),
                depends_on_id: "dep-b".to_string(),
                edge_type: "blocks".to_string(),
                occurred_at: "3".to_string(),
            },
            TaskAggregateEvent::TaskDependencyRemoved {
                task_id: "task".to_string(),
                depends_on_id: "dep-b".to_string(),
                edge_type: "blocks".to_string(),
                occurred_at: "4".to_string(),
            },
        ];

        let projection = replay_task_events(&events);
        assert_eq!(
            projection.dependencies["task"],
            vec![TaskAggregateDependencyEdge {
                depends_on_id: "dep-a".to_string(),
                edge_type: "parent-child".to_string(),
            }]
        );
    }

    #[test]
    fn task_aggregate_plans_parent_auto_close_event() {
        let mut parent =
            TaskAggregateTaskSnapshot::closed("parent", "100", Some("root".to_string()));
        parent.close_reason =
            Some("all direct child tasks closed after closing `child`".to_string());

        let plan = plan_close_task(TaskCloseCommand {
            task: TaskAggregateTaskSnapshot::closed("child", "100", Some("parent".to_string())),
            reason: "proof passed".to_string(),
            occurred_at: "100".to_string(),
            auto_closed_parents: vec![parent],
        });

        assert_eq!(
            plan.events[1],
            TaskAggregateEvent::ParentAutoClosed {
                task_id: "parent".to_string(),
                reason: "all direct child tasks closed after closing `child`".to_string(),
                occurred_at: "100".to_string(),
                source_child_id: "child".to_string(),
            }
        );
        assert_eq!(plan.touched_task_ids, vec!["child", "parent"]);
    }

    #[test]
    fn task_aggregate_plans_status_update_and_parent_reopen_events() {
        let mut parent =
            TaskAggregateTaskSnapshot::closed("parent", "101", Some("root".to_string()));
        parent.status = "in_progress".to_string();
        parent.closed_at = None;
        parent.close_reason = None;

        let plan = plan_update_task_status(TaskStatusUpdateCommand {
            task: TaskAggregateTaskSnapshot {
                id: "child".to_string(),
                status: "in_progress".to_string(),
                updated_at: "101".to_string(),
                closed_at: None,
                close_reason: None,
                parent_id: Some("parent".to_string()),
            },
            occurred_at: "101".to_string(),
            auto_closed_parents: Vec::new(),
            auto_reopened_parents: vec![parent],
        });

        assert_eq!(
            plan.events,
            vec![
                TaskAggregateEvent::TaskStatusUpdated {
                    task_id: "child".to_string(),
                    status: "in_progress".to_string(),
                    occurred_at: "101".to_string(),
                },
                TaskAggregateEvent::ParentAutoReopened {
                    task_id: "parent".to_string(),
                    occurred_at: "101".to_string(),
                    source_child_id: "child".to_string(),
                },
            ]
        );
        assert_eq!(plan.touched_task_ids, vec!["child", "parent"]);
    }

    #[test]
    fn task_aggregate_plans_create_task_and_parent_reopen_events() {
        let mut parent =
            TaskAggregateTaskSnapshot::closed("parent", "102", Some("root".to_string()));
        parent.status = "in_progress".to_string();
        parent.closed_at = None;
        parent.close_reason = None;

        let plan = plan_create_task(TaskCreateCommand {
            task: TaskAggregateTaskSnapshot {
                id: "child".to_string(),
                status: "open".to_string(),
                updated_at: "102".to_string(),
                closed_at: None,
                close_reason: None,
                parent_id: Some("parent".to_string()),
            },
            occurred_at: "102".to_string(),
            auto_reopened_parents: vec![parent],
        });

        assert_eq!(
            plan.events,
            vec![
                TaskAggregateEvent::TaskCreated {
                    task_id: "child".to_string(),
                    status: "open".to_string(),
                    parent_id: Some("parent".to_string()),
                    occurred_at: "102".to_string(),
                },
                TaskAggregateEvent::ParentAutoReopened {
                    task_id: "parent".to_string(),
                    occurred_at: "102".to_string(),
                    source_child_id: "child".to_string(),
                },
            ]
        );
        assert_eq!(plan.touched_task_ids, vec!["child", "parent"]);
    }

    #[test]
    fn task_aggregate_plans_reparent_and_source_parent_auto_close_events() {
        let mut source_parent =
            TaskAggregateTaskSnapshot::closed("source", "103", Some("root".to_string()));
        source_parent.close_reason =
            Some("all direct child tasks moved from `source` to `target`".to_string());

        let plan = plan_reparent_tasks(TaskReparentCommand {
            moved_tasks: vec![TaskAggregateTaskSnapshot {
                id: "child".to_string(),
                status: "open".to_string(),
                updated_at: "103".to_string(),
                closed_at: None,
                close_reason: None,
                parent_id: Some("target".to_string()),
            }],
            from_parent_id: "source".to_string(),
            to_parent_id: "target".to_string(),
            occurred_at: "103".to_string(),
            auto_closed_parents: vec![source_parent],
        });

        assert_eq!(
            plan.events[0],
            TaskAggregateEvent::TaskReparented {
                task_id: "child".to_string(),
                from_parent_id: "source".to_string(),
                to_parent_id: "target".to_string(),
                occurred_at: "103".to_string(),
            }
        );
        assert_eq!(
            plan.events[1],
            TaskAggregateEvent::ParentAutoClosed {
                task_id: "source".to_string(),
                reason: "all direct child tasks moved from `source` to `target`".to_string(),
                occurred_at: "103".to_string(),
                source_child_id: "source".to_string(),
            }
        );
        assert_eq!(plan.touched_task_ids, vec!["child", "source", "target"]);
    }

    #[test]
    fn task_aggregate_replays_events_into_status_and_parent_projection() {
        let create = plan_create_task(TaskCreateCommand {
            task: TaskAggregateTaskSnapshot {
                id: "child".to_string(),
                status: "open".to_string(),
                updated_at: "100".to_string(),
                closed_at: None,
                close_reason: None,
                parent_id: Some("source".to_string()),
            },
            occurred_at: "100".to_string(),
            auto_reopened_parents: Vec::new(),
        });
        let reparent = plan_reparent_tasks(TaskReparentCommand {
            moved_tasks: vec![TaskAggregateTaskSnapshot {
                id: "child".to_string(),
                status: "open".to_string(),
                updated_at: "101".to_string(),
                closed_at: None,
                close_reason: None,
                parent_id: Some("target".to_string()),
            }],
            from_parent_id: "source".to_string(),
            to_parent_id: "target".to_string(),
            occurred_at: "101".to_string(),
            auto_closed_parents: Vec::new(),
        });
        let close = plan_close_task(TaskCloseCommand {
            task: TaskAggregateTaskSnapshot::closed("child", "102", Some("target".to_string())),
            reason: "done".to_string(),
            occurred_at: "102".to_string(),
            auto_closed_parents: Vec::new(),
        });
        let events = create
            .events
            .into_iter()
            .chain(reparent.events)
            .chain(close.events)
            .collect::<Vec<_>>();

        let projection = replay_task_events(&events);

        assert_eq!(
            projection.statuses.get("child"),
            Some(&"closed".to_string())
        );
        assert_eq!(
            projection.parent_ids.get("child"),
            Some(&Some("target".to_string()))
        );
    }

    #[test]
    fn task_aggregate_plans_dependency_add_and_remove_events() {
        let add = plan_add_task_dependency(TaskDependencyMutationCommand {
            task_id: "task-a".to_string(),
            depends_on_id: "task-b".to_string(),
            edge_type: "blocks".to_string(),
            occurred_at: "104".to_string(),
        });
        let remove = plan_remove_task_dependency(TaskDependencyMutationCommand {
            task_id: "task-a".to_string(),
            depends_on_id: "task-b".to_string(),
            edge_type: "blocks".to_string(),
            occurred_at: "105".to_string(),
        });

        assert_eq!(
            add.events,
            vec![TaskAggregateEvent::TaskDependencyAdded {
                task_id: "task-a".to_string(),
                depends_on_id: "task-b".to_string(),
                edge_type: "blocks".to_string(),
                occurred_at: "104".to_string(),
            }]
        );
        assert_eq!(add.touched_task_ids, vec!["task-a", "task-b"]);
        assert_eq!(
            remove.events,
            vec![TaskAggregateEvent::TaskDependencyRemoved {
                task_id: "task-a".to_string(),
                depends_on_id: "task-b".to_string(),
                edge_type: "blocks".to_string(),
                occurred_at: "105".to_string(),
            }]
        );
        assert_eq!(remove.touched_task_ids, vec!["task-a", "task-b"]);
    }

    #[test]
    fn task_aggregate_replays_dependency_events() {
        let add = plan_add_task_dependency(TaskDependencyMutationCommand {
            task_id: "task-a".to_string(),
            depends_on_id: "task-b".to_string(),
            edge_type: "blocks".to_string(),
            occurred_at: "104".to_string(),
        });
        let remove = plan_remove_task_dependency(TaskDependencyMutationCommand {
            task_id: "task-a".to_string(),
            depends_on_id: "task-b".to_string(),
            edge_type: "blocks".to_string(),
            occurred_at: "105".to_string(),
        });

        let added_projection = replay_task_events(&add.events);
        assert_eq!(
            added_projection.dependencies.get("task-a"),
            Some(&vec![TaskAggregateDependencyEdge {
                depends_on_id: "task-b".to_string(),
                edge_type: "blocks".to_string(),
            }])
        );

        let removed_projection = replay_task_events(
            &add.events
                .into_iter()
                .chain(remove.events)
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            removed_projection.dependencies.get("task-a"),
            Some(&Vec::new())
        );
    }

    #[test]
    fn task_aggregate_plans_metadata_update_event() {
        let plan = plan_update_task_metadata(TaskMetadataUpdateCommand {
            task_id: "task-a".to_string(),
            occurred_at: "106".to_string(),
        });

        assert_eq!(
            plan.events,
            vec![TaskAggregateEvent::TaskMetadataUpdated {
                task_id: "task-a".to_string(),
                occurred_at: "106".to_string(),
            }]
        );
        assert_eq!(plan.touched_task_ids, vec!["task-a"]);
    }

    #[test]
    fn task_aggregate_plan_coverage_blocks_persistence_without_events() {
        let persisted_task_ids = BTreeSet::from(["task-a".to_string()]);
        let error = ensure_task_mutation_plan_covers_persistence(
            &TaskMutationPlan {
                events: Vec::new(),
                mutations: vec![TaskAggregateMutation::SetTaskStatus {
                    task_id: "task-a".to_string(),
                    status: "closed".to_string(),
                    updated_at: "100".to_string(),
                    closed_at: Some("100".to_string()),
                    close_reason: Some("done".to_string()),
                }],
                touched_task_ids: vec!["task-a".to_string()],
            },
            &persisted_task_ids,
        )
        .expect_err("empty event plan must not cover persistence");

        assert_eq!(error.blocker_code, TASK_AGGREGATE_PLAN_BLOCKER_EMPTY_EVENTS);
    }

    #[test]
    fn task_aggregate_plan_coverage_blocks_persistence_touch_mismatch() {
        let plan = plan_add_task_dependency(TaskDependencyMutationCommand {
            task_id: "task-a".to_string(),
            depends_on_id: "task-b".to_string(),
            edge_type: "blocks".to_string(),
            occurred_at: "104".to_string(),
        });
        let error = ensure_task_mutation_plan_covers_persistence(
            &plan,
            &BTreeSet::from(["task-a".to_string()]),
        )
        .expect_err("dependency plan must include both touched task ids");

        assert_eq!(
            error.blocker_code,
            TASK_AGGREGATE_PLAN_BLOCKER_TOUCH_MISMATCH
        );
        assert_eq!(error.actual_task_ids, vec!["task-a", "task-b"]);
    }
}
