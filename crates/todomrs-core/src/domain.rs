use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Completed,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Priority {
    None,
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reminder {
    pub id: Uuid,
    pub task_id: Uuid,
    pub remind_at: DateTime<Utc>,
    pub status: ReminderStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReminderStatus {
    Pending,
    Triggered,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    }

    #[test]
    fn test_task_completion() {
        let user_id = Uuid::new_v4();
        let mut task = Task::new(user_id, "Test task".to_string());

        task.complete();

        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
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
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn is_overdue(&self) -> bool {
        if let Some(due) = self.due_at {
            self.status == TaskStatus::Pending && due < Utc::now()
        } else {
            false
        }
    }
}
