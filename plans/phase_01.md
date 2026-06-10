# Phase 1: Project Scaffold + Domain Model + SQLite Setup

## Session Goal

Initialize the Rust workspace structure, define the core domain model types (Task, Project, Tag, Reminder, RecurrenceRule), and set up SQLite with the initial database schema. By the end of this session, you should have a compilable Rust workspace with well-defined types and a working SQLite database with migrations.

## Expected Outcome

- Cargo workspace with 4 crates: `todomrs-core`, `todomrs-store`, `todomrs-sync`, `todomrs-tui`
- Core domain types defined with proper derives (Debug, Clone, Serialize, Deserialize, PartialEq)
- SQLite database with initial schema (tasks, projects, tags, reminders, recurrence_rules tables)
- Migration system working (sqlx)
- Unit tests for domain types passing
- `cargo build` succeeds
- `cargo test` passes

## Context

Read `~/Projects/TodoRS/Northstar.md` first to understand the product vision.

This is the foundation phase. Everything else builds on these types and the database schema. Do not rush. Get the types right. The domain model must match what's described in Northstar.md.

## Prerequisites

- Rust toolchain installed (rustup, cargo)
- SQLite3 installed
- sqlx-cli installed (`cargo install sqlx-cli --no-default-features --features sqlite`)

If not installed, run:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install sqlx-cli --no-default-features --features sqlite
```

## Tasks

### Task 1: Initialize Cargo Workspace

**Objective:** Create the workspace structure with 4 crates.

**Steps:**

1. Navigate to project root:
```bash
cd ~/Projects/TodoRS
```

2. Create workspace Cargo.toml at root:
```toml
[workspace]
members = [
    "crates/todomrs-core",
    "crates/todomrs-store",
    "crates/todomrs-sync",
    "crates/todomrs-tui",
]
resolver = "2"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
thiserror = "1.0"
anyhow = "1.0"
```

3. Create crate directories:
```bash
mkdir -p crates/todomrs-core/src
mkdir -p crates/todomrs-store/src
mkdir -p crates/todomrs-sync/src
mkdir -p crates/todomrs-tui/src
```

4. Create Cargo.toml for each crate:

**crates/todomrs-core/Cargo.toml:**
```toml
[package]
name = "todomrs-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
```

**crates/todomrs-store/Cargo.toml:**
```toml
[package]
name = "todomrs-store"
version = "0.1.0"
edition = "2021"

[dependencies]
todomrs-core = { path = "../todomrs-core" }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "chrono", "uuid"] }
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
```

**crates/todomrs-sync/Cargo.toml:**
```toml
[package]
name = "todomrs-sync"
version = "0.1.0"
edition = "2021"

[dependencies]
todomrs-core = { path = "../todomrs-core" }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }

[dev-dependencies]
```

**crates/todomrs-tui/Cargo.toml:**
```toml
[package]
name = "todomrs-tui"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "todomrs"
path = "src/main.rs"

[dependencies]
todomrs-core = { path = "../todomrs-core" }
todomrs-store = { path = "../todomrs-store" }
todomrs-sync = { path = "../todomrs-sync" }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
ratatui = "0.26"
crossterm = "0.27"
tokio = { version = "1.0", features = ["full"] }
clap = { version = "4.0", features = ["derive"] }

[dev-dependencies]
```

5. Create minimal lib.rs for each library crate:

**crates/todomrs-core/src/lib.rs:**
```rust
pub mod domain;
```

**crates/todomrs-store/src/lib.rs:**
```rust
pub mod db;
```

**crates/todomrs-sync/src/lib.rs:**
```rust
pub mod operations;
```

**crates/todomrs-tui/src/main.rs:**
```rust
fn main() {
    println!("TodoRS TUI - Phase 1 scaffold complete");
}
```

6. Create empty module files:
```bash
touch crates/todomrs-core/src/domain.rs
touch crates/todomrs-store/src/db.rs
touch crates/todomrs-sync/src/operations.rs
```

7. Verify workspace compiles:
```bash
cargo build
```

Expected: Build succeeds with no errors (warnings about unused code are OK).

**Commit:**
```bash
git init
git add .
git commit -m "feat: initialize Cargo workspace with 4 crates"
```

---

### Task 2: Define Core Domain Types

**Objective:** Define Task, Project, Tag, Reminder, RecurrenceRule types in todomrs-core.

**Steps:**

1. Open `crates/todomrs-core/src/domain.rs`

2. Define the types:

```rust
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
```

3. Add helper methods for Task:

```rust
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
```

4. Verify it compiles:
```bash
cargo build
```

Expected: Success.

**Commit:**
```bash
git add .
git commit -m "feat: define core domain types (Task, Project, Tag, Reminder, RecurrenceRule)"
```

---

### Task 3: Write Unit Tests for Domain Types

**Objective:** Write tests for Task creation, completion, and overdue logic.

**Steps:**

1. Add tests module to `crates/todomrs-core/src/domain.rs`:

```rust
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
```

2. Run tests:
```bash
cargo test -p todomrs-core
```

Expected: 3 tests pass.

**Commit:**
```bash
git add .
git commit -m "test: add unit tests for Task domain type"
```

---

### Task 4: Set Up SQLite with sqlx

**Objective:** Configure SQLite database connection and create initial migration.

**Steps:**

1. Create migrations directory:
```bash
mkdir -p migrations
```

2. Create initial migration file:
```bash
sqlx migrate add init -s
```

This creates `migrations/<timestamp>_init.sql`.

3. Edit the migration file to create tables:

```sql
-- Create users table (for future multi-user support)
CREATE TABLE users (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT UNIQUE NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create projects table
CREATE TABLE projects (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    archived_at TEXT
);

-- Create tags table
CREATE TABLE tags (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create recurrence_rules table
CREATE TABLE recurrence_rules (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    interval INTEGER NOT NULL DEFAULT 1,
    by_weekday TEXT,
    by_monthday TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create tasks table
CREATE TABLE tasks (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    tag_ids TEXT NOT NULL DEFAULT '[]',
    priority TEXT NOT NULL DEFAULT 'none',
    due_at TEXT,
    scheduled_at TEXT,
    recurrence_rule_id TEXT REFERENCES recurrence_rules(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    deleted_at TEXT
);

-- Create reminders table
CREATE TABLE reminders (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    remind_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create indexes
CREATE INDEX idx_tasks_user_id ON tasks(user_id);
CREATE INDEX idx_tasks_project_id ON tasks(project_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_due_at ON tasks(due_at);
CREATE INDEX idx_reminders_task_id ON reminders(task_id);
CREATE INDEX idx_reminders_remind_at ON reminders(remind_at);
```

4. Create database and apply migration:
```bash
export DATABASE_URL="sqlite:./todomrs.db"
sqlx database create
sqlx migrate run
```

Expected: Database created, migration applied successfully.

5. Verify tables exist:
```bash
sqlite3 todomrs.db ".tables"
```

Expected output:
```
projects       recurrence_rules  tags           users
reminders      tasks
```

**Commit:**
```bash
git add .
git commit -m "feat: add initial SQLite migration with all tables"
```

---

### Task 5: Create Database Connection Module

**Objective:** Create a database connection wrapper in todomrs-store.

**Steps:**

1. Edit `crates/todomrs-store/src/db.rs`:

```rust
use anyhow::Result;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}
```

2. Update `crates/todomrs-store/src/lib.rs`:

```rust
pub mod db;

pub use db::Database;
```

3. Verify it compiles:
```bash
cargo build
```

Expected: Success.

**Commit:**
```bash
git add .
git commit -m "feat: add Database connection wrapper in todomrs-store"
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
- All tests pass (3 tests in todomrs-core)
- No critical clippy warnings

Verify database exists:
```bash
ls -lh todomrs.db
sqlite3 todomrs.db ".schema"
```

Expected: Database file exists, schema shows all tables.

## Pitfalls

1. **Don't skip the workspace setup.** Multiple crates are essential for clean architecture.

2. **Don't use raw SQL strings in Rust code.** Use sqlx migrations and compile-time checked queries later.

3. **Don't add features you don't need yet.** Keep types minimal. We'll add more fields later if needed.

4. **Don't forget to commit after each task.** Small commits make debugging easier.

5. **Don't ignore test failures.** If tests fail, fix them before moving to the next task.

## Handoff to Next Phase

Phase 2 will assume:
- Workspace compiles cleanly
- Domain types are defined and tested
- SQLite database exists with all tables
- Database connection wrapper works

Phase 2 will implement the store layer: CRUD operations for tasks, projects, and tags using sqlx.
