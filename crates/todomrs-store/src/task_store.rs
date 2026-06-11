use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use todomrs_core::domain::Task;
use uuid::Uuid;

pub struct TaskStore {
    pool: SqlitePool,
}

impl TaskStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn create(&self, task: &Task) -> Result<()> {
        let mut tx = self.pool.begin().await.context("begin transaction")?;

        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, user_id, title, description, status, project_id,
                priority, due_at, scheduled_at, recurrence_rule_id,
                created_at, updated_at, completed_at, deleted_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(task.id)
        .bind(task.user_id)
        .bind(&task.title)
        .bind(&task.description)
        .bind(serialize_enum(&task.status)?)
        .bind(task.project_id)
        .bind(serialize_enum(&task.priority)?)
        .bind(task.due_at)
        .bind(task.scheduled_at)
        .bind(task.recurrence_rule_id)
        .bind(task.created_at)
        .bind(task.updated_at)
        .bind(task.completed_at)
        .bind(task.deleted_at)
        .execute(&mut *tx)
        .await?;

        // Insert task_tags junction rows in the same transaction
        set_task_tags(&mut tx, task.id, &task.tag_ids).await?;

        tx.commit().await.context("commit transaction")?;
        Ok(())
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Task>> {
        let row: Option<TaskRow> = sqlx::query_as(
            "SELECT * FROM tasks WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let tag_ids = get_task_tags_by_id(&self.pool, id).await?;
                Ok(Some(r.into_task(tag_ids)?))
            }
            None => Ok(None),
        }
    }

    pub async fn get_all(&self, user_id: Uuid) -> Result<Vec<Task>> {
        let rows: Vec<TaskRow> = sqlx::query_as(
            "SELECT * FROM tasks WHERE user_id = ? AND deleted_at IS NULL ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // Collect all task IDs to batch-load tags
        let task_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let tag_map = get_task_tags_batch(&self.pool, &task_ids).await?;

        let mut tasks = Vec::with_capacity(rows.len());
        for r in rows {
            let tags = tag_map.get(&r.id).cloned().unwrap_or_default();
            tasks.push(r.into_task(tags)?);
        }
        Ok(tasks)
    }

    pub async fn update(&self, task: &Task) -> Result<()> {
        let mut tx = self.pool.begin().await.context("begin transaction")?;

        sqlx::query(
            r#"
            UPDATE tasks SET
                title = ?, description = ?, status = ?, project_id = ?,
                priority = ?, due_at = ?, scheduled_at = ?,
                recurrence_rule_id = ?, updated_at = ?,
                completed_at = ?, deleted_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&task.title)
        .bind(&task.description)
        .bind(serialize_enum(&task.status)?)
        .bind(task.project_id)
        .bind(serialize_enum(&task.priority)?)
        .bind(task.due_at)
        .bind(task.scheduled_at)
        .bind(task.recurrence_rule_id)
        .bind(task.updated_at)
        .bind(task.completed_at)
        .bind(task.deleted_at)
        .bind(task.id)
        .execute(&mut *tx)
        .await?;

        // Replace task_tags in the same transaction
        set_task_tags(&mut tx, task.id, &task.tag_ids).await?;

        tx.commit().await.context("commit transaction")?;
        Ok(())
    }

    /// Permanently remove a task and its tag associations (hard delete).
    pub async fn hard_delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Soft-delete a task by setting `deleted_at`. It will be excluded from `get_all`.
    pub async fn soft_delete(&self, id: Uuid) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE tasks SET deleted_at = ?, updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

// ── Free functions for tag management ───────────────────────────────────

/// Replace all tag associations for a task with the given list,
/// using a mutable connection reference (from a transaction or pool).
async fn set_task_tags(
    conn: &mut sqlx::SqliteConnection,
    task_id: Uuid,
    tag_ids: &[Uuid],
) -> Result<()> {
    // Clear existing
    sqlx::query("DELETE FROM task_tags WHERE task_id = ?")
        .bind(task_id)
        .execute(&mut *conn)
        .await?;

    // Insert new
    for &tag_id in tag_ids {
        sqlx::query("INSERT INTO task_tags (task_id, tag_id) VALUES (?, ?)")
            .bind(task_id)
            .bind(tag_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

/// Fetch tag IDs for a single task.
async fn get_task_tags_by_id(pool: &SqlitePool, task_id: Uuid) -> Result<Vec<Uuid>> {
    let rows: Vec<(Uuid,)> =
        sqlx::query_as("SELECT tag_id FROM task_tags WHERE task_id = ? ORDER BY tag_id")
            .bind(task_id)
            .fetch_all(pool)
            .await?;

    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Batch-load tag IDs for multiple tasks, returning a map.
async fn get_task_tags_batch(
    pool: &SqlitePool,
    task_ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, Vec<Uuid>>> {
    use std::collections::HashMap;

    if task_ids.is_empty() {
        return Ok(HashMap::new());
    }

    // For SQLite we build placeholders
    let placeholders: Vec<String> = task_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let sql = format!(
        "SELECT task_id, tag_id FROM task_tags WHERE task_id IN ({}) ORDER BY task_id, tag_id",
        placeholders.join(", ")
    );

    let mut query = sqlx::query_as::<_, (Uuid, Uuid)>(&sql);
    for &id in task_ids {
        query = query.bind(id);
    }

    let rows: Vec<(Uuid, Uuid)> = query.fetch_all(pool).await?;

    let mut map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (task_id, tag_id) in rows {
        map.entry(task_id).or_default().push(tag_id);
    }
    Ok(map)
}

// ── Row structs ─────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: Uuid,
    user_id: Uuid,
    title: String,
    description: Option<String>,
    status: String,
    project_id: Option<Uuid>,
    priority: String,
    due_at: Option<DateTime<Utc>>,
    scheduled_at: Option<DateTime<Utc>>,
    recurrence_rule_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
}

impl TaskRow {
    fn into_task(self, tag_ids: Vec<Uuid>) -> Result<Task> {
        Ok(Task {
            id: self.id,
            user_id: self.user_id,
            title: self.title,
            description: self.description,
            status: deserialize_enum(&self.status)?,
            project_id: self.project_id,
            tag_ids,
            priority: deserialize_enum(&self.priority)?,
            due_at: self.due_at,
            scheduled_at: self.scheduled_at,
            recurrence_rule_id: self.recurrence_rule_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            completed_at: self.completed_at,
            deleted_at: self.deleted_at,
        })
    }
}

// ── Enum serialization helpers ─────────────────────────────────────────

fn serialize_enum<T: serde::Serialize>(value: &T) -> Result<String> {
    // serde_json serializes snake_case enums as e.g. "\"pending\""
    // We want just the inner string without JSON quotes
    let json = serde_json::to_value(value).context("enum serialization")?;
    let s = json.as_str().ok_or_else(|| anyhow::anyhow!("enum did not serialize to a string"))?;
    Ok(s.to_string())
}

fn deserialize_enum<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    // DB stores bare strings like "pending", serde expects JSON "\"pending\""
    let json = format!("\"{}\"", s);
    serde_json::from_str(&json).context("enum deserialization")
}
