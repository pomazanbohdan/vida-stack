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
}
