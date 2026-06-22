use serde::{Deserialize, Serialize};

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
        occurred_at: String,
    },
    TaskReparented {
        task_id: String,
        from_parent_id: String,
        to_parent_id: String,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMutationPlan {
    pub events: Vec<TaskAggregateEvent>,
    pub mutations: Vec<TaskAggregateMutation>,
    pub touched_task_ids: Vec<String>,
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
}
