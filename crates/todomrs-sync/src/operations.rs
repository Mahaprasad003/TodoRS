use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use todomrs_core::domain::{Priority, Task, TaskStatus};
use uuid::Uuid;

/// An immutable operation representing a single change to the data model.
///
/// Operations are the foundation of the sync protocol. Every mutation
/// (create, update, delete) produces one operation, which is stored,
/// synced between devices, and applied idempotently on the receiving end.
///
/// The `seq` field provides a monotonically increasing per-device sequence
/// number that guarantees ordering for conflict-free merge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Operation {
    pub op_id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub seq: i64,
    pub entity: Entity,
    pub entity_id: Uuid,
    pub op_type: OperationType,
    pub payload: OperationPayload,
    pub created_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
}

/// The kind of entity this operation targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Entity {
    Task,
    Project,
    Tag,
    Reminder,
    RecurrenceRule,
}

/// Whether the operation creates, updates, or deletes the target entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    Create,
    Update,
    Delete,
}

/// The payload carrying the specific fields changed by this operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationPayload {
    TaskCreate {
        title: String,
        description: Option<String>,
        status: TaskStatus,
        project_id: Option<Uuid>,
        tag_ids: Vec<Uuid>,
        priority: Priority,
        due_at: Option<DateTime<Utc>>,
        scheduled_at: Option<DateTime<Utc>>,
        recurrence_rule_id: Option<Uuid>,
    },
    TaskUpdate {
        title: Option<String>,
        description: Option<String>,
        status: Option<TaskStatus>,
        project_id: Option<Uuid>,
        tag_ids: Option<Vec<Uuid>>,
        priority: Option<Priority>,
        due_at: Option<DateTime<Utc>>,
        scheduled_at: Option<DateTime<Utc>>,
        recurrence_rule_id: Option<Uuid>,
        completed_at: Option<DateTime<Utc>>,
    },
    ProjectCreate {
        name: String,
        color: Option<String>,
        sort_order: i32,
    },
    ProjectUpdate {
        name: Option<String>,
        color: Option<String>,
        sort_order: Option<i32>,
        archived_at: Option<DateTime<Utc>>,
    },
    TagCreate {
        name: String,
        color: Option<String>,
    },
    TagUpdate {
        name: Option<String>,
        color: Option<String>,
    },
    Delete,
}

impl Operation {
    pub fn create_task(user_id: Uuid, device_id: Uuid, seq: i64, task: &Task) -> Self {
        Self {
            op_id: Uuid::new_v4(),
            user_id,
            device_id,
            seq,
            entity: Entity::Task,
            entity_id: task.id,
            op_type: OperationType::Create,
            payload: OperationPayload::TaskCreate {
                title: task.title.clone(),
                description: task.description.clone(),
                status: task.status.clone(),
                project_id: task.project_id,
                tag_ids: task.tag_ids.clone(),
                priority: task.priority.clone(),
                due_at: task.due_at,
                scheduled_at: task.scheduled_at,
                recurrence_rule_id: task.recurrence_rule_id,
            },
            created_at: Utc::now(),
            synced_at: None,
        }
    }

    pub fn update_task_title(
        user_id: Uuid,
        device_id: Uuid,
        seq: i64,
        task_id: Uuid,
        new_title: String,
    ) -> Self {
        Self {
            op_id: Uuid::new_v4(),
            user_id,
            device_id,
            seq,
            entity: Entity::Task,
            entity_id: task_id,
            op_type: OperationType::Update,
            payload: OperationPayload::TaskUpdate {
                title: Some(new_title),
                description: None,
                status: None,
                project_id: None,
                tag_ids: None,
                priority: None,
                due_at: None,
                scheduled_at: None,
                recurrence_rule_id: None,
                completed_at: None,
            },
            created_at: Utc::now(),
            synced_at: None,
        }
    }

    pub fn complete_task(user_id: Uuid, device_id: Uuid, seq: i64, task_id: Uuid) -> Self {
        Self {
            op_id: Uuid::new_v4(),
            user_id,
            device_id,
            seq,
            entity: Entity::Task,
            entity_id: task_id,
            op_type: OperationType::Update,
            payload: OperationPayload::TaskUpdate {
                title: None,
                description: None,
                status: Some(TaskStatus::Completed),
                project_id: None,
                tag_ids: None,
                priority: None,
                due_at: None,
                scheduled_at: None,
                recurrence_rule_id: None,
                completed_at: Some(Utc::now()),
            },
            created_at: Utc::now(),
            synced_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_task_operation() {
        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();
        let task = Task::new(user_id, "Test task".to_string());

        let op = Operation::create_task(user_id, device_id, 1, &task);

        assert_eq!(op.entity, Entity::Task);
        assert_eq!(op.entity_id, task.id);
        assert_eq!(op.op_type, OperationType::Create);
        assert_eq!(op.seq, 1);

        if let OperationPayload::TaskCreate { ref title, .. } = op.payload {
            assert_eq!(title, "Test task");
        } else {
            panic!("Expected TaskCreate payload");
        }
    }

    #[test]
    fn test_complete_task_operation() {
        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();

        let op = Operation::complete_task(user_id, device_id, 2, task_id);

        assert_eq!(op.entity, Entity::Task);
        assert_eq!(op.entity_id, task_id);
        assert_eq!(op.op_type, OperationType::Update);

        if let OperationPayload::TaskUpdate {
            ref status,
            ref completed_at,
            ..
        } = op.payload
        {
            assert_eq!(*status, Some(TaskStatus::Completed));
            assert!(completed_at.is_some());
        } else {
            panic!("Expected TaskUpdate payload");
        }
    }

    #[test]
    fn test_update_task_title_operation() {
        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();

        let op =
            Operation::update_task_title(user_id, device_id, 3, task_id, "New title".to_string());

        assert_eq!(op.entity, Entity::Task);
        assert_eq!(op.entity_id, task_id);
        assert_eq!(op.op_type, OperationType::Update);

        if let OperationPayload::TaskUpdate { ref title, .. } = op.payload {
            assert_eq!(title.as_deref(), Some("New title"));
        } else {
            panic!("Expected TaskUpdate payload");
        }
    }
}
