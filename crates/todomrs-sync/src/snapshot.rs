use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use todomrs_core::domain::{Project, Tag, Task};
use uuid::Uuid;

/// A point-in-time snapshot of a user's entire data set.
///
/// Snapshots compact a sequence of operations into a single state,
/// enabling efficient bootstrap for new devices and recovery from
/// operation log tail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub snapshot_seq: i64,
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub tags: Vec<Tag>,
    pub created_at: DateTime<Utc>,
}

impl Snapshot {
    pub fn new(
        user_id: Uuid,
        device_id: Uuid,
        snapshot_seq: i64,
        tasks: Vec<Task>,
        projects: Vec<Project>,
        tags: Vec<Tag>,
    ) -> Self {
        Self {
            user_id,
            device_id,
            snapshot_seq,
            tasks,
            projects,
            tags,
            created_at: Utc::now(),
        }
    }
}
