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

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn create(&self, project: &Project) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO projects (
                id, user_id, name, color, sort_order,
                created_at, updated_at, archived_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(project.id)
        .bind(project.user_id)
        .bind(&project.name)
        .bind(&project.color)
        .bind(project.sort_order)
        .bind(project.created_at)
        .bind(project.updated_at)
        .bind(project.archived_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Project>> {
        let row: Option<ProjectRow> = sqlx::query_as(
            "SELECT * FROM projects WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(ProjectRow::into_project))
    }

    pub async fn get_all(&self, user_id: Uuid) -> Result<Vec<Project>> {
        let rows: Vec<ProjectRow> = sqlx::query_as(
            "SELECT * FROM projects WHERE user_id = ? AND archived_at IS NULL ORDER BY sort_order",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(ProjectRow::into_project).collect())
    }

    pub async fn update(&self, project: &Project) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE projects SET
                name = ?, color = ?, sort_order = ?,
                updated_at = ?, archived_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&project.name)
        .bind(&project.color)
        .bind(project.sort_order)
        .bind(project.updated_at)
        .bind(project.archived_at)
        .bind(project.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Permanently remove a project (hard delete).
    pub async fn hard_delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Soft-delete a project by setting `archived_at`.
    /// Find a project by name for a given user (returns None if archived or not found).
    pub async fn find_by_name(&self, user_id: Uuid, name: &str) -> Result<Option<Project>> {
        let row: Option<ProjectRow> = sqlx::query_as(
            "SELECT * FROM projects WHERE user_id = ? AND name = ? AND archived_at IS NULL",
        )
        .bind(user_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(ProjectRow::into_project))
    }

    pub async fn soft_delete(&self, id: Uuid) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE projects SET archived_at = ?, updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    color: Option<String>,
    sort_order: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

impl ProjectRow {
    fn into_project(self) -> Project {
        Project {
            id: self.id,
            user_id: self.user_id,
            name: self.name,
            color: self.color,
            sort_order: self.sort_order,
            created_at: self.created_at,
            updated_at: self.updated_at,
            archived_at: self.archived_at,
        }
    }
}
