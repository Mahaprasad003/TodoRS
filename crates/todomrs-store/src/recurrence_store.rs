use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use todomrs_core::domain::RecurrenceRule;
use uuid::Uuid;

pub struct RecurrenceRuleStore {
    pool: SqlitePool,
}

impl RecurrenceRuleStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, rule: &RecurrenceRule) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO recurrence_rules (
                id, task_id, kind, interval, by_weekday, by_monthday,
                timezone, wait_for_completion, anchor_mode,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(rule.id)
        .bind(rule.task_id)
        .bind(serialize_enum(&rule.kind)?)
        .bind(rule.interval)
        .bind(format_comma_separated_i32(&rule.by_weekday))
        .bind(format_comma_separated_i32(&rule.by_monthday))
        .bind(&rule.timezone)
        .bind(rule.wait_for_completion as i32)
        .bind(serialize_enum(&rule.anchor_mode)?)
        .bind(rule.created_at)
        .bind(rule.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<RecurrenceRule>> {
        let row: Option<RecurrenceRuleRow> = sqlx::query_as(
            "SELECT * FROM recurrence_rules WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.into_rule()?)),
            None => Ok(None),
        }
    }

    pub async fn get_all(&self) -> Result<Vec<RecurrenceRule>> {
        let rows: Vec<RecurrenceRuleRow> = sqlx::query_as(
            "SELECT * FROM recurrence_rules ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut rules = Vec::with_capacity(rows.len());
        for r in rows {
            rules.push(r.into_rule()?);
        }
        Ok(rules)
    }

    pub async fn find_by_task_id(&self, task_id: Uuid) -> Result<Option<RecurrenceRule>> {
        let row: Option<RecurrenceRuleRow> = sqlx::query_as(
            "SELECT * FROM recurrence_rules WHERE task_id = ?",
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.into_rule()?)),
            None => Ok(None),
        }
    }

    pub async fn update(&self, rule: &RecurrenceRule) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE recurrence_rules SET
                task_id = ?, kind = ?, interval = ?,
                by_weekday = ?, by_monthday = ?, timezone = ?,
                wait_for_completion = ?, anchor_mode = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(rule.task_id)
        .bind(serialize_enum(&rule.kind)?)
        .bind(rule.interval)
        .bind(format_comma_separated_i32(&rule.by_weekday))
        .bind(format_comma_separated_i32(&rule.by_monthday))
        .bind(&rule.timezone)
        .bind(rule.wait_for_completion as i32)
        .bind(serialize_enum(&rule.anchor_mode)?)
        .bind(rule.updated_at)
        .bind(rule.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM recurrence_rules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

// ── Row struct ─────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct RecurrenceRuleRow {
    id: Uuid,
    task_id: Uuid,
    kind: String,
    interval: i32,
    by_weekday: Option<String>,
    by_monthday: Option<String>,
    timezone: String,
    wait_for_completion: i32,
    anchor_mode: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl RecurrenceRuleRow {
    fn into_rule(self) -> Result<RecurrenceRule> {
        Ok(RecurrenceRule {
            id: self.id,
            task_id: self.task_id,
            kind: deserialize_enum(&self.kind)?,
            interval: self.interval,
            by_weekday: parse_comma_separated_i32(self.by_weekday),
            by_monthday: parse_comma_separated_i32(self.by_monthday),
            timezone: self.timezone,
            wait_for_completion: self.wait_for_completion != 0,
            anchor_mode: deserialize_enum(&self.anchor_mode)?,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

// ── Serialization helpers ──────────────────────────────────────────────

fn serialize_enum<T: serde::Serialize>(value: &T) -> Result<String> {
    let json = serde_json::to_value(value).context("enum serialization")?;
    let s = json
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("enum did not serialize to a string"))?;
    Ok(s.to_string())
}

fn deserialize_enum<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    let json = format!("\"{}\"", s);
    serde_json::from_str(&json).context("enum deserialization")
}

fn parse_comma_separated_i32(s: Option<String>) -> Option<Vec<i32>> {
    s.filter(|s| !s.is_empty()).map(|s| {
        s.split(',')
            .filter_map(|part| part.trim().parse::<i32>().ok())
            .collect()
    })
}

fn format_comma_separated_i32(v: &Option<Vec<i32>>) -> Option<String> {
    v.as_ref().map(|v| {
        v.iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",")
    })
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use todomrs_core::domain::{AnchorMode, RecurrenceKind};
    use sqlx::SqlitePool;

    async fn setup_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create pool");
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("run migrations");
        pool
    }

    /// Create a user and task row so FK constraints are satisfied.
    async fn setup_with_task(pool: &SqlitePool) -> Uuid {
        let user_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query("INSERT INTO users (id, email) VALUES (?, ?)")
            .bind(user_id)
            .bind(format!("{}@test.com", user_id))
            .execute(pool)
            .await
            .expect("insert user");

        sqlx::query(
            "INSERT INTO tasks (id, user_id, title, status, priority, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(task_id)
        .bind(user_id)
        .bind("test task")
        .bind("pending")
        .bind("none")
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert task");

        task_id
    }

    #[tokio::test]
    async fn test_create_recurrence_rule() {
        let pool = setup_pool().await;
        let store = RecurrenceRuleStore::new(pool.clone());

        let task_id = setup_with_task(&pool).await;
        let rule = RecurrenceRule {
            id: Uuid::new_v4(),
            task_id,
            kind: RecurrenceKind::Daily,
            interval: 1,
            by_weekday: None,
            by_monthday: None,
            timezone: "UTC".to_string(),
            wait_for_completion: false,
            anchor_mode: AnchorMode::Schedule,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        store.create(&rule).await.unwrap();

        let loaded = store.find_by_task_id(task_id).await.unwrap().unwrap();
        assert_eq!(loaded.kind, RecurrenceKind::Daily);
        assert_eq!(loaded.interval, 1);
        assert_eq!(loaded.wait_for_completion, false);
        assert_eq!(loaded.anchor_mode, AnchorMode::Schedule);
        assert_eq!(loaded.timezone, "UTC");
    }

    #[tokio::test]
    async fn test_create_recurrence_rule_with_completion_anchor() {
        let pool = setup_pool().await;
        let store = RecurrenceRuleStore::new(pool.clone());

        let task_id = setup_with_task(&pool).await;
        let rule = RecurrenceRule {
            id: Uuid::new_v4(),
            task_id,
            kind: RecurrenceKind::Weekly,
            interval: 2,
            by_weekday: None,
            by_monthday: None,
            timezone: "UTC".to_string(),
            wait_for_completion: true,
            anchor_mode: AnchorMode::Completion,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        store.create(&rule).await.unwrap();

        let loaded = store.get_by_id(rule.id).await.unwrap().unwrap();
        assert_eq!(loaded.kind, RecurrenceKind::Weekly);
        assert_eq!(loaded.interval, 2);
        assert_eq!(loaded.wait_for_completion, true);
        assert_eq!(loaded.anchor_mode, AnchorMode::Completion);
    }

    #[tokio::test]
    async fn test_get_all_recurrence_rules() {
        let pool = setup_pool().await;
        let store = RecurrenceRuleStore::new(pool.clone());

        let task_id1 = setup_with_task(&pool).await;
        let task_id2 = setup_with_task(&pool).await;

        let rule1 = RecurrenceRule {
            id: Uuid::new_v4(),
            task_id: task_id1,
            kind: RecurrenceKind::Daily,
            interval: 1,
            by_weekday: None,
            by_monthday: None,
            timezone: "UTC".to_string(),
            wait_for_completion: false,
            anchor_mode: AnchorMode::Schedule,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let rule2 = RecurrenceRule {
            id: Uuid::new_v4(),
            task_id: task_id2,
            kind: RecurrenceKind::Monthly,
            interval: 3,
            by_weekday: None,
            by_monthday: None,
            timezone: "UTC".to_string(),
            wait_for_completion: true,
            anchor_mode: AnchorMode::Completion,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        store.create(&rule1).await.unwrap();
        store.create(&rule2).await.unwrap();

        let all = store.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_update_recurrence_rule() {
        let pool = setup_pool().await;
        let store = RecurrenceRuleStore::new(pool.clone());

        let task_id = setup_with_task(&pool).await;
        let mut rule = RecurrenceRule {
            id: Uuid::new_v4(),
            task_id,
            kind: RecurrenceKind::Daily,
            interval: 1,
            by_weekday: None,
            by_monthday: None,
            timezone: "UTC".to_string(),
            wait_for_completion: false,
            anchor_mode: AnchorMode::Schedule,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        store.create(&rule).await.unwrap();

        rule.interval = 3;
        rule.wait_for_completion = true;
        store.update(&rule).await.unwrap();

        let loaded = store.find_by_task_id(task_id).await.unwrap().unwrap();
        assert_eq!(loaded.interval, 3);
        assert_eq!(loaded.wait_for_completion, true);
    }

    #[tokio::test]
    async fn test_delete_recurrence_rule() {
        let pool = setup_pool().await;
        let store = RecurrenceRuleStore::new(pool.clone());

        let task_id = setup_with_task(&pool).await;
        let rule = RecurrenceRule {
            id: Uuid::new_v4(),
            task_id,
            kind: RecurrenceKind::Daily,
            interval: 1,
            by_weekday: None,
            by_monthday: None,
            timezone: "UTC".to_string(),
            wait_for_completion: false,
            anchor_mode: AnchorMode::Schedule,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        store.create(&rule).await.unwrap();
        store.delete(rule.id).await.unwrap();

        let loaded = store.get_by_id(rule.id).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_find_by_task_id_not_found() {
        let pool = setup_pool().await;
        let store = RecurrenceRuleStore::new(pool);

        let result = store.find_by_task_id(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }
}
