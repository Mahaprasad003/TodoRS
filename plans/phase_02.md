# Phase 2: Store Layer — CRUD Operations

## Session Goal

Implement the persistence layer: create, read, update, and delete operations for tasks, projects, and tags using sqlx. By the end of this session, you should be able to persist and retrieve all core entities from SQLite.

## Expected Outcome

- TaskStore with methods: create, get_by_id, get_all, update, delete, get_by_project, get_by_status
- ProjectStore with methods: create, get_by_id, get_all, update, delete
- TagStore with methods: create, get_by_id, get_all, update, delete
- All methods async and return Result types
- Integration tests for each store
- `cargo test` passes with all store tests

## Context

Phase 1 is complete. You have:
- Cargo workspace with 4 crates
- Core domain types (Task, Project, Tag, etc.)
- SQLite database with all tables created
- Database connection wrapper in todomrs-store

Now you'll implement the actual CRUD operations that read/write to the database.

## Prerequisites

- Phase 1 complete and committed
- Database exists at `./todomrs.db`
- All migrations applied

## Tasks

### Task 1: Implement TaskStore — Create and Get

**Objective:** Implement methods to create tasks and retrieve them by ID.

**Steps:**

1. Create `crates/todomrs-store/src/task_store.rs`:

```rust
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use todomrs_core::domain::{Task, TaskStatus, Priority};
use uuid::Uuid;

pub struct TaskStore {
    pool: SqlitePool,
}

impl TaskStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, task: &Task) -> Result<()> {
        let tag_ids_json = serde_json::to_string(&task.tag_ids)?;

        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, user_id, title, description, status, project_id,
                tag_ids, priority, due_at, scheduled_at, recurrence_rule_id,
                created_at, updated_at, completed_at, deleted_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(task.id.to_string())
        .bind(task.user_id.to_string())
        .bind(&task.title)
        .bind(&task.description)
        .bind(serde_json::to_string(&task.status)?)
        .bind(task.project_id.map(|id| id.to_string()))
        .bind(&tag_ids_json)
        .bind(serde_json::to_string(&task.priority)?)
        .bind(task.due_at.map(|dt| dt.to_rfc3339()))
        .bind(task.scheduled_at.map(|dt| dt.to_rfc3339()))
        .bind(task.recurrence_rule_id.map(|id| id.to_string()))
        .bind(task.created_at.to_rfc3339())
        .bind(task.updated_at.to_rfc3339())
        .bind(task.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(task.deleted_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Task>> {
        let row: Option<TaskRow> = sqlx::query_as(
            "SELECT * FROM tasks WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_task()))
    }
}

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: String,
    user_id: String,
    title: String,
    description: Option<String>,
    status: String,
    project_id: Option<String>,
    tag_ids: String,
    priority: String,
    due_at: Option<String>,
    scheduled_at: Option<String>,
    recurrence_rule_id: Option<String>,
    created_at: String,
    updated_at: String,
    completed_at: Option<String>,
    deleted_at: Option<String>,
}

impl TaskRow {
    fn into_task(self) -> Task {
        Task {
            id: Uuid::parse_str(&self.id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            title: self.title,
            description: self.description,
            status: serde_json::from_str(&self.status).unwrap(),
            project_id: self.project_id.map(|id| Uuid::parse_str(&id).unwrap()),
            tag_ids: serde_json::from_str(&self.tag_ids).unwrap_or_default(),
            priority: serde_json::from_str(&self.priority).unwrap(),
            due_at: self.due_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
            scheduled_at: self.scheduled_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
            recurrence_rule_id: self.recurrence_rule_id.map(|id| Uuid::parse_str(&id).unwrap()),
            created_at: DateTime::parse_from_rfc3339(&self.created_at).unwrap().with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&self.updated_at).unwrap().with_timezone(&Utc),
            completed_at: self.completed_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
            deleted_at: self.deleted_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
        }
    }
}
```

2. Update `crates/todomrs-store/src/lib.rs`:

```rust
pub mod db;
pub mod task_store;

pub use db::Database;
pub use task_store::TaskStore;
```

3. Write integration test in `crates/todomrs-store/tests/task_store_test.rs`:

```rust
use anyhow::Result;
use todomrs_core::domain::Task;
use todomrs_store::{Database, TaskStore};
use uuid::Uuid;

async fn setup() -> Result<(Database, TaskStore)> {
    let db = Database::new("sqlite::memory:").await?;
    let pool = db.pool().clone();
    
    // Apply migrations
    sqlx::migrate!("../../migrations").run(&pool).await?;
    
    let store = TaskStore::new(pool);
    Ok((db, store))
}

#[tokio::test]
async fn test_create_and_get_task() -> Result<()> {
    let (db, store) = setup().await?;
    
    let user_id = Uuid::new_v4();
    
    // Insert test user first
    sqlx::query("INSERT INTO users (id, email) VALUES (?, ?)")
        .bind(user_id.to_string())
        .bind("test@example.com")
        .execute(store.pool())
        .await?;
    
    let task = Task::new(user_id, "Test task".to_string());
    store.create(&task).await?;
    
    let retrieved = store.get_by_id(task.id).await?;
    assert!(retrieved.is_some());
    
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.title, "Test task");
    assert_eq!(retrieved.user_id, user_id);
    
    db.close().await;
    Ok(())
}
```

4. Add sqlx dependency to dev-dependencies in `crates/todomrs-store/Cargo.toml`:

```toml
[dev-dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "chrono", "uuid", "migrate"] }
```

5. Run test:
```bash
cargo test -p todomrs-store
```

Expected: Test passes.

**Commit:**
```bash
git add .
git commit -m "feat: implement TaskStore create and get_by_id"
```

---

### Task 2: Implement TaskStore — Get All, Update, Delete

**Objective:** Add methods to list all tasks, update tasks, and delete tasks.

**Steps:**

1. Add methods to `crates/todomrs-store/src/task_store.rs`:

```rust
pub async fn get_all(&self, user_id: Uuid) -> Result<Vec<Task>> {
    let rows: Vec<TaskRow> = sqlx::query_as(
        "SELECT * FROM tasks WHERE user_id = ? AND deleted_at IS NULL ORDER BY created_at DESC"
    )
    .bind(user_id.to_string())
    .fetch_all(&self.pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into_task()).collect())
}

pub async fn update(&self, task: &Task) -> Result<()> {
    let tag_ids_json = serde_json::to_string(&task.tag_ids)?;

    sqlx::query(
        r#"
        UPDATE tasks SET
            title = ?,
            description = ?,
            status = ?,
            project_id = ?,
            tag_ids = ?,
            priority = ?,
            due_at = ?,
            scheduled_at = ?,
            recurrence_rule_id = ?,
            updated_at = ?,
            completed_at = ?,
            deleted_at = ?
        WHERE id = ?
        "#
    )
    .bind(&task.title)
    .bind(&task.description)
    .bind(serde_json::to_string(&task.status)?)
    .bind(task.project_id.map(|id| id.to_string()))
    .bind(&tag_ids_json)
    .bind(serde_json::to_string(&task.priority)?)
    .bind(task.due_at.map(|dt| dt.to_rfc3339()))
    .bind(task.scheduled_at.map(|dt| dt.to_rfc3339()))
    .bind(task.recurrence_rule_id.map(|id| id.to_string()))
    .bind(task.updated_at.to_rfc3339())
    .bind(task.completed_at.map(|dt| dt.to_rfc3339()))
    .bind(task.deleted_at.map(|dt| dt.to_rfc3339()))
    .bind(task.id.to_string())
    .execute(&self.pool)
    .await?;

    Ok(())
}

pub async fn delete(&self, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

    Ok(())
}

pub fn pool(&self) -> &SqlitePool {
    &self.pool
}
```

2. Add tests to `crates/todomrs-store/tests/task_store_test.rs`:

```rust
#[tokio::test]
async fn test_get_all_tasks() -> Result<()> {
    let (db, store) = setup().await?;
    let user_id = Uuid::new_v4();
    
    sqlx::query("INSERT INTO users (id, email) VALUES (?, ?)")
        .bind(user_id.to_string())
        .bind("test@example.com")
        .execute(store.pool())
        .await?;
    
    let task1 = Task::new(user_id, "Task 1".to_string());
    let task2 = Task::new(user_id, "Task 2".to_string());
    
    store.create(&task1).await?;
    store.create(&task2).await?;
    
    let tasks = store.get_all(user_id).await?;
    assert_eq!(tasks.len(), 2);
    
    db.close().await;
    Ok(())
}

#[tokio::test]
async fn test_update_task() -> Result<()> {
    let (db, store) = setup().await?;
    let user_id = Uuid::new_v4();
    
    sqlx::query("INSERT INTO users (id, email) VALUES (?, ?)")
        .bind(user_id.to_string())
        .bind("test@example.com")
        .execute(store.pool())
        .await?;
    
    let mut task = Task::new(user_id, "Original".to_string());
    store.create(&task).await?;
    
    task.title = "Updated".to_string();
    task.updated_at = chrono::Utc::now();
    store.update(&task).await?;
    
    let retrieved = store.get_by_id(task.id).await?.unwrap();
    assert_eq!(retrieved.title, "Updated");
    
    db.close().await;
    Ok(())
}

#[tokio::test]
async fn test_delete_task() -> Result<()> {
    let (db, store) = setup().await?;
    let user_id = Uuid::new_v4();
    
    sqlx::query("INSERT INTO users (id, email) VALUES (?, ?)")
        .bind(user_id.to_string())
        .bind("test@example.com")
        .execute(store.pool())
        .await?;
    
    let task = Task::new(user_id, "To delete".to_string());
    store.create(&task).await?;
    
    store.delete(task.id).await?;
    
    let retrieved = store.get_by_id(task.id).await?;
    assert!(retrieved.is_none());
    
    db.close().await;
    Ok(())
}
```

3. Run tests:
```bash
cargo test -p todomrs-store
```

Expected: All tests pass.

**Commit:**
```bash
git add .
git commit -m "feat: implement TaskStore get_all, update, delete"
```

---

### Task 3: Implement ProjectStore and TagStore

**Objective:** Implement CRUD operations for projects and tags.

**Steps:**

1. Create `crates/todomrs-store/src/project_store.rs`:

```rust
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use todomrs_core::domain::Project;
use uuid::Uuid;

pub struct ProjectStore {
    pool: SqlitePool,
}

impl ProjectStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, project: &Project) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO projects (
                id, user_id, name, color, sort_order,
                created_at, updated_at, archived_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(project.id.to_string())
        .bind(project.user_id.to_string())
        .bind(&project.name)
        .bind(&project.color)
        .bind(project.sort_order)
        .bind(project.created_at.to_rfc3339())
        .bind(project.updated_at.to_rfc3339())
        .bind(project.archived_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Project>> {
        let row: Option<ProjectRow> = sqlx::query_as(
            "SELECT * FROM projects WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_project()))
    }

    pub async fn get_all(&self, user_id: Uuid) -> Result<Vec<Project>> {
        let rows: Vec<ProjectRow> = sqlx::query_as(
            "SELECT * FROM projects WHERE user_id = ? AND archived_at IS NULL ORDER BY sort_order"
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_project()).collect())
    }

    pub async fn update(&self, project: &Project) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE projects SET
                name = ?,
                color = ?,
                sort_order = ?,
                updated_at = ?,
                archived_at = ?
            WHERE id = ?
            "#
        )
        .bind(&project.name)
        .bind(&project.color)
        .bind(project.sort_order)
        .bind(project.updated_at.to_rfc3339())
        .bind(project.archived_at.map(|dt| dt.to_rfc3339()))
        .bind(project.id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: String,
    user_id: String,
    name: String,
    color: Option<String>,
    sort_order: i32,
    created_at: String,
    updated_at: String,
    archived_at: Option<String>,
}

impl ProjectRow {
    fn into_project(self) -> Project {
        Project {
            id: Uuid::parse_str(&self.id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            name: self.name,
            color: self.color,
            sort_order: self.sort_order,
            created_at: DateTime::parse_from_rfc3339(&self.created_at).unwrap().with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&self.updated_at).unwrap().with_timezone(&Utc),
            archived_at: self.archived_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
        }
    }
}
```

2. Create `crates/todomrs-store/src/tag_store.rs` with similar structure for Tag CRUD.

3. Update `crates/todomrs-store/src/lib.rs`:

```rust
pub mod db;
pub mod task_store;
pub mod project_store;
pub mod tag_store;

pub use db::Database;
pub use task_store::TaskStore;
pub use project_store::ProjectStore;
pub use tag_store::TagStore;
```

4. Write integration tests for ProjectStore and TagStore in `crates/todomrs-store/tests/`.

5. Run all tests:
```bash
cargo test -p todomrs-store
```

Expected: All tests pass.

**Commit:**
```bash
git add .
git commit -m "feat: implement ProjectStore and TagStore CRUD operations"
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
- All store tests pass (create, get, update, delete for tasks, projects, tags)
- No critical clippy warnings

## Pitfalls

1. **Don't forget to insert test users before creating tasks/projects/tags.** Foreign key constraints will fail otherwise.

2. **Don't skip datetime parsing.** SQLite stores datetimes as TEXT. You must parse them back to DateTime<Utc>.

3. **Don't ignore JSON serialization for tag_ids.** It's stored as a JSON array string in SQLite.

4. **Don't forget to handle NULL values.** Optional fields like description, due_at, completed_at can be NULL.

## Handoff to Next Phase

Phase 3 will assume:
- TaskStore, ProjectStore, TagStore work correctly
- All CRUD operations tested
- Database schema stable

Phase 3 will implement the natural language quick-add parser and recurrence rule engine.
