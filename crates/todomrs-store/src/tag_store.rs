use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use todomrs_core::domain::Tag;
use uuid::Uuid;

pub struct TagStore {
    pool: SqlitePool,
}

impl TagStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn create(&self, tag: &Tag) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO tags (id, user_id, name, color, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(tag.id)
        .bind(tag.user_id)
        .bind(&tag.name)
        .bind(&tag.color)
        .bind(tag.created_at)
        .bind(tag.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Tag>> {
        let row: Option<TagRow> = sqlx::query_as(
            "SELECT * FROM tags WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(TagRow::into_tag))
    }

    pub async fn get_all(&self, user_id: Uuid) -> Result<Vec<Tag>> {
        let rows: Vec<TagRow> = sqlx::query_as(
            "SELECT * FROM tags WHERE user_id = ? ORDER BY name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(TagRow::into_tag).collect())
    }

    pub async fn update(&self, tag: &Tag) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE tags SET name = ?, color = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&tag.name)
        .bind(&tag.color)
        .bind(tag.updated_at)
        .bind(tag.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM tags WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct TagRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    color: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TagRow {
    fn into_tag(self) -> Tag {
        Tag {
            id: self.id,
            user_id: self.user_id,
            name: self.name,
            color: self.color,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
