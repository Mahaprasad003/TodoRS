# Phase 4: Operation Log + Snapshot/Replay

## Session Goal

Implement the operation log system that tracks all changes as immutable operations, enabling sync between devices. Create the operation types, local operation queue, and snapshot/replay mechanism.

## Expected Outcome

- Operation types defined (CreateTask, UpdateTask, DeleteTask, etc.)
- Operation log stored in SQLite
- Methods to append operations and query unsynced operations
- Snapshot mechanism to compact old operations
- Replay mechanism to rebuild state from snapshot + operations
- Comprehensive tests for operation log
- `cargo test` passes with all operation log tests

## Context

Phase 3 is complete. You have:
- Natural language parser working
- Recurrence engine calculating next occurrences
- Task creation from natural language input

Now you'll add the sync foundation: operation log. This is the core of the sync system. Every change creates an operation. Operations are synced between devices.

## Prerequisites

- Phase 3 complete and committed
- Core domain types defined
- Store layer working

## Tasks

### Task 1: Define Operation Types

**Objective:** Create operation types that represent all possible changes to tasks, projects, and tags.

**Steps:**

1. Create `crates/todomrs-sync/src/operations.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use todomrs_core::domain::{Priority, TaskStatus};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Entity {
    Task,
    Project,
    Tag,
    Reminder,
    RecurrenceRule,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationType {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
```

2. Add helper methods:

```rust
impl Operation {
    pub fn create_task(
        user_id: Uuid,
        device_id: Uuid,
        seq: i64,
        task: &todomrs_core::domain::Task,
    ) -> Self {
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

    pub fn complete_task(
        user_id: Uuid,
        device_id: Uuid,
        seq: i64,
        task_id: Uuid,
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
```

3. Add tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use todomrs_core::domain::Task;

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

        if let OperationPayload::TaskCreate { title, .. } = op.payload {
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

        if let OperationPayload::TaskUpdate { status, completed_at, .. } = op.payload {
            assert_eq!(status, Some(TaskStatus::Completed));
            assert!(completed_at.is_some());
        } else {
            panic!("Expected TaskUpdate payload");
        }
    }
}
```

4. Verify it compiles:
```bash
cargo build
cargo test -p todomrs-sync
```

Expected: Tests pass.

**Commit:**
```bash
git add .
git commit -m "feat: define operation types for sync (Task, Project, Tag operations)"
```

---

### Task 2: Create Operations Table and Store

**Objective:** Add operations table to database and implement operation storage.

**Steps:**

1. Create new migration:
```bash
sqlx migrate add operations
```

2. Edit the migration file:

```sql
-- Create operations table for sync
CREATE TABLE operations (
    op_id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    op_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL,
    synced_at TEXT
);

-- Create index for efficient querying
CREATE INDEX idx_operations_user_device_seq ON operations(user_id, device_id, seq);
CREATE INDEX idx_operations_synced_at ON operations(synced_at);

-- Create sync_state table to track last synced sequence
CREATE TABLE sync_state (
    user_id TEXT PRIMARY KEY NOT NULL,
    device_id TEXT NOT NULL,
    last_local_seq INTEGER NOT NULL DEFAULT 0,
    last_synced_seq INTEGER NOT NULL DEFAULT 0,
    last_sync_at TEXT
);
```

3. Apply migration:
```bash
sqlx migrate run
```

4. Create `crates/todomrs-store/src/operation_store.rs`:

```rust
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use todomrs_sync::operations::{Entity, Operation, OperationPayload, OperationType};
use uuid::Uuid;

pub struct OperationStore {
    pool: SqlitePool,
}

impl OperationStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn append(&self, op: &Operation) -> Result<()> {
        let payload_json = serde_json::to_string(&op.payload)?;
        let entity_str = serde_json::to_string(&op.entity)?;
        let op_type_str = serde_json::to_string(&op.op_type)?;

        sqlx::query(
            r#"
            INSERT INTO operations (
                op_id, user_id, device_id, seq, entity, entity_id,
                op_type, payload, created_at, synced_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(op.op_id.to_string())
        .bind(op.user_id.to_string())
        .bind(op.device_id.to_string())
        .bind(op.seq)
        .bind(&entity_str)
        .bind(op.entity_id.to_string())
        .bind(&op_type_str)
        .bind(&payload_json)
        .bind(op.created_at.to_rfc3339())
        .bind(op.synced_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_unsynced(&self, user_id: Uuid) -> Result<Vec<Operation>> {
        let rows: Vec<OperationRow> = sqlx::query_as(
            "SELECT * FROM operations WHERE user_id = ? AND synced_at IS NULL ORDER BY seq ASC"
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_operation()).collect::<Result<Vec<_>>>()?)
    }

    pub async fn mark_synced(&self, op_ids: &[Uuid]) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let op_id_strings: Vec<String> = op_ids.iter().map(|id| id.to_string()).collect();

        for op_id in op_id_strings {
            sqlx::query("UPDATE operations SET synced_at = ? WHERE op_id = ?")
                .bind(&now)
                .bind(&op_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    pub async fn get_next_seq(&self, user_id: Uuid, device_id: Uuid) -> Result<i64> {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT MAX(seq) FROM operations WHERE user_id = ? AND device_id = ?"
        )
        .bind(user_id.to_string())
        .bind(device_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|(max_seq,)| max_seq).unwrap_or(0) + 1)
    }
}

#[derive(sqlx::FromRow)]
struct OperationRow {
    op_id: String,
    user_id: String,
    device_id: String,
    seq: i64,
    entity: String,
    entity_id: String,
    op_type: String,
    payload: String,
    created_at: String,
    synced_at: Option<String>,
}

impl OperationRow {
    fn into_operation(self) -> Result<Operation> {
        Ok(Operation {
            op_id: Uuid::parse_str(&self.op_id)?,
            user_id: Uuid::parse_str(&self.user_id)?,
            device_id: Uuid::parse_str(&self.device_id)?,
            seq: self.seq,
            entity: serde_json::from_str(&self.entity)?,
            entity_id: Uuid::parse_str(&self.entity_id)?,
            op_type: serde_json::from_str(&self.op_type)?,
            payload: serde_json::from_str(&self.payload)?,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)?.with_timezone(&Utc),
            synced_at: self.synced_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
        })
    }
}
```

5. Update `crates/todomrs-store/src/lib.rs`:

```rust
pub mod db;
pub mod task_store;
pub mod project_store;
pub mod tag_store;
pub mod operation_store;

pub use db::Database;
pub use task_store::TaskStore;
pub use project_store::ProjectStore;
pub use tag_store::TagStore;
pub use operation_store::OperationStore;
```

6. Add tests in `crates/todomrs-store/tests/operation_store_test.rs`:

```rust
use anyhow::Result;
use todomrs_core::domain::Task;
use todomrs_store::{Database, OperationStore};
use todomrs_sync::operations::Operation;
use uuid::Uuid;

async fn setup() -> Result<(Database, OperationStore)> {
    let db = Database::new("sqlite::memory:").await?;
    let pool = db.pool().clone();
    sqlx::migrate!("../../migrations").run(&pool).await?;
    let store = OperationStore::new(pool);
    Ok((db, store))
}

#[tokio::test]
async fn test_append_and_get_unsynced() -> Result<()> {
    let (db, store) = setup().await?;
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    let task = Task::new(user_id, "Test".to_string());
    let op = Operation::create_task(user_id, device_id, 1, &task);

    store.append(&op).await?;

    let unsynced = store.get_unsynced(user_id).await?;
    assert_eq!(unsynced.len(), 1);
    assert_eq!(unsynced[0].op_id, op.op_id);

    db.close().await;
    Ok(())
}

#[tokio::test]
async fn test_mark_synced() -> Result<()> {
    let (db, store) = setup().await?;
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    let task = Task::new(user_id, "Test".to_string());
    let op = Operation::create_task(user_id, device_id, 1, &task);

    store.append(&op).await?;
    store.mark_synced(&[op.op_id]).await?;

    let unsynced = store.get_unsynced(user_id).await?;
    assert_eq!(unsynced.len(), 0);

    db.close().await;
    Ok(())
}

#[tokio::test]
async fn test_get_next_seq() -> Result<()> {
    let (db, store) = setup().await?;
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    let seq1 = store.get_next_seq(user_id, device_id).await?;
    assert_eq!(seq1, 1);

    let task = Task::new(user_id, "Test".to_string());
    let op = Operation::create_task(user_id, device_id, 1, &task);
    store.append(&op).await?;

    let seq2 = store.get_next_seq(user_id, device_id).await?;
    assert_eq!(seq2, 2);

    db.close().await;
    Ok(())
}
```

7. Run tests:
```bash
cargo test -p todomrs-store
```

Expected: All operation store tests pass.

**Commit:**
```bash
git add .
git commit -m "feat: implement operation store for sync log"
```

---

### Task 3: Implement Snapshot and Replay

**Objective:** Create a snapshot mechanism to compact old operations and replay mechanism to rebuild state.

**Steps:**

1. Add snapshot table to migration:

```sql
-- Create snapshots table
CREATE TABLE snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    snapshot_seq INTEGER NOT NULL,
    state_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_snapshots_user_device ON snapshots(user_id, device_id, snapshot_seq);
```

2. Apply migration:
```bash
sqlx migrate run
```

3. Create `crates/todomrs-sync/src/snapshot.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use todomrs_core::domain::{Project, Tag, Task};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
```

4. Update `crates/todomrs-sync/src/lib.rs`:

```rust
pub mod operations;
pub mod snapshot;

pub use operations::*;
pub use snapshot::Snapshot;
```

5. Add snapshot/replay methods to OperationStore:

```rust
use todomrs_sync::Snapshot;

pub async fn create_snapshot(
    &self,
    user_id: Uuid,
    device_id: Uuid,
    snapshot_seq: i64,
    tasks: Vec<todomrs_core::domain::Task>,
    projects: Vec<todomrs_core::domain::Project>,
    tags: Vec<todomrs_core::domain::Tag>,
) -> Result<()> {
    let snapshot = Snapshot::new(user_id, device_id, snapshot_seq, tasks, projects, tags);
    let state_json = serde_json::to_string(&snapshot)?;

    sqlx::query(
        "INSERT INTO snapshots (user_id, device_id, snapshot_seq, state_json) VALUES (?, ?, ?, ?)"
    )
    .bind(user_id.to_string())
    .bind(device_id.to_string())
    .bind(snapshot_seq)
    .bind(&state_json)
    .execute(&self.pool)
    .await?;

    Ok(())
}

pub async fn get_latest_snapshot(&self, user_id: Uuid) -> Result<Option<Snapshot>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT state_json FROM snapshots WHERE user_id = ? ORDER BY snapshot_seq DESC LIMIT 1"
    )
    .bind(user_id.to_string())
    .fetch_optional(&self.pool)
    .await?;

    Ok(row.and_then(|(json,)| serde_json::from_str(&json).ok()))
}
```

6. Add tests for snapshot/replay.

7. Run tests:
```bash
cargo test -p todomrs-store
```

Expected: All tests pass.

**Commit:**
```bash
git add .
git commit -m "feat: implement snapshot and replay mechanism for sync"
```

---

## Verification

Run all checks:

```bash
cargo build
cargo test
cargo clippy
```

Expected:
- Build succeeds
- All operation log and snapshot tests pass
- No critical clippy warnings

## Pitfalls

1. **Don't skip operation ordering.** Operations must be applied in sequence order.

2. **Don't forget to mark operations as synced.** Otherwise they'll be re-uploaded.

3. **Don't store too many unsynced operations.** Create snapshots periodically.

4. **Don't ignore operation conflicts.** We'll handle them in Phase 8.

## Handoff to Next Phase

Phase 5 will assume:
- Operation log system working
- Operations can be appended and queried
- Snapshot/replay mechanism in place

Phase 5 will build the TUI skeleton with ratatui.
