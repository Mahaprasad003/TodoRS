use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Task {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub project_id: Option<Uuid>,
    pub tag_ids: Vec<Uuid>,
    pub priority: Priority,
    pub due_at: Option<DateTime<Utc>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub recurrence_rule_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    None,
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Project {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Tag {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Reminder {
    pub id: Uuid,
    pub task_id: Uuid,
    pub remind_at: DateTime<Utc>,
    pub status: ReminderStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderStatus {
    Pending,
    Triggered,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RecurrenceRule {
    pub id: Uuid,
    pub task_id: Uuid,
    pub kind: RecurrenceKind,
    pub interval: i32,
    pub by_weekday: Option<Vec<i32>>,
    pub by_monthday: Option<Vec<i32>>,
    pub timezone: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurrenceKind {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_task_creation() {
        let user_id = Uuid::new_v4();
        let task = Task::new(user_id, "Test task".to_string());

        assert_eq!(task.title, "Test task");
        assert_eq!(task.user_id, user_id);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.completed_at.is_none());
        assert!(task.deleted_at.is_none());
        // Verify idempotency of new()
        assert!(task.created_at <= Utc::now());
        assert_eq!(task.created_at, task.updated_at);
    }

    #[test]
    fn test_task_completion() {
        let user_id = Uuid::new_v4();
        let mut task = Task::new(user_id, "Test task".to_string());

        let pre_update = task.updated_at;
        // Brief pause to ensure updated_at changes
        std::thread::sleep(std::time::Duration::from_millis(1));

        task.complete();

        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
        assert!(task.updated_at > pre_update);
    }

    #[test]
    fn test_task_completion_idempotent() {
        let mut task = Task::new(Uuid::new_v4(), "Test".to_string());
        task.complete();
        let completed_at = task.completed_at;
        let updated_at = task.updated_at;

        // Completing again should be a no-op
        task.complete();
        assert_eq!(task.completed_at, completed_at);
        assert_eq!(task.updated_at, updated_at);
    }

    #[test]
    fn test_task_overdue() {
        let user_id = Uuid::new_v4();
        let mut task = Task::new(user_id, "Test task".to_string());

        // Not overdue without due date
        assert!(!task.is_overdue());

        // Not overdue with future due date
        task.due_at = Some(Utc::now() + Duration::hours(1));
        assert!(!task.is_overdue());

        // Overdue with past due date
        task.due_at = Some(Utc::now() - Duration::hours(1));
        assert!(task.is_overdue());

        // Not overdue if completed
        task.complete();
        assert!(!task.is_overdue());
    }

    #[test]
    fn test_task_overdue_boundary() {
        let mut task = Task::new(Uuid::new_v4(), "Test".to_string());
        // Due very recently (1 nanosecond as far back as chrono allows) — strict < means
        // this is overdue. We verify tiny past IS overdue, verifying the strict inequality.
        task.due_at = Some(Utc::now() - Duration::microseconds(1));
        assert!(task.is_overdue());
    }

    #[test]
    fn test_task_overdue_deleted() {
        let mut task = Task::new(Uuid::new_v4(), "Test".to_string());
        task.due_at = Some(Utc::now() - Duration::hours(1));
        task.delete();
        // Deleted task should not be overdue even if due_at is in the past
        assert!(!task.is_overdue());
    }

    #[test]
    fn test_task_complete_after_delete_noop() {
        let mut task = Task::new(Uuid::new_v4(), "Test".to_string());
        task.delete();
        assert!(task.is_deleted());

        // Completing a deleted task should be a no-op
        task.complete();
        assert_ne!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_task_uncomplete() {
        let mut task = Task::new(Uuid::new_v4(), "Test".to_string());
        task.complete();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());

        task.uncomplete();
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_task_delete() {
        let mut task = Task::new(Uuid::new_v4(), "Test".to_string());
        assert!(!task.is_deleted());

        task.delete();
        assert!(task.is_deleted());
        assert!(task.deleted_at.is_some());
    }

    #[test]
    fn test_task_serde_roundtrip() {
        let task = Task::new(Uuid::new_v4(), "Serde test".to_string());
        let json = serde_json::to_string(&task).expect("serialize");
        let deserialized: Task = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(task, deserialized);
    }

    #[test]
    fn test_priority_serde_snake_case() {
        // Verify enum serializes as snake_case to match DB defaults
        let json = serde_json::to_string(&Priority::None).unwrap();
        assert_eq!(json, "\"none\"");
        let json = serde_json::to_string(&Priority::High).unwrap();
        assert_eq!(json, "\"high\"");
    }

    #[test]
    fn test_task_status_serde_snake_case() {
        let json = serde_json::to_string(&TaskStatus::Pending).unwrap();
        assert_eq!(json, "\"pending\"");
        let json = serde_json::to_string(&TaskStatus::Completed).unwrap();
        assert_eq!(json, "\"completed\"");
    }
}

impl Task {
    pub fn new(user_id: Uuid, title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            title,
            description: None,
            status: TaskStatus::Pending,
            project_id: None,
            tag_ids: Vec::new(),
            priority: Priority::None,
            due_at: None,
            scheduled_at: None,
            recurrence_rule_id: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            deleted_at: None,
        }
    }

    pub fn complete(&mut self) {
        // Silently no-op if task is already deleted
        if self.deleted_at.is_some() {
            return;
        }
        // Silently no-op if already completed (preserves original completed_at)
        if self.status == TaskStatus::Completed {
            return;
        }
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn uncomplete(&mut self) {
        self.status = TaskStatus::Pending;
        self.completed_at = None;
        self.updated_at = Utc::now();
    }

    pub fn delete(&mut self) {
        self.deleted_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    pub fn is_overdue(&self) -> bool {
        if self.deleted_at.is_some() {
            return false;
        }
        if let Some(due) = self.due_at {
            self.status == TaskStatus::Pending && due < Utc::now()
        } else {
            false
        }
    }
}
