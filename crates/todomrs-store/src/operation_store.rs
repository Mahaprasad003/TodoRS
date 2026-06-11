/// Store for the operation log and snapshots used by the sync protocol.
///
/// Operations are immutable records of every data change. They are
/// stored in SQLite and synced between devices. Snapshots provide
/// point-in-time compaction of the operation log for efficient
/// bootstrap of new devices.
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use todomrs_sync::operations::Operation;
use todomrs_sync::snapshot::Snapshot;
use uuid::Uuid;

pub struct OperationStore {
    pool: SqlitePool,
}

impl OperationStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn append(&self, op: &Operation) -> Result<()> {
        let payload_json = serde_json::to_string(&op.payload).context("serialize payload")?;
        let entity_str = serde_json::to_string(&op.entity).context("serialize entity")?;
        let op_type_str = serde_json::to_string(&op.op_type).context("serialize op_type")?;

        sqlx::query(
            r#"
            INSERT INTO operations (
                op_id, user_id, device_id, seq, entity, entity_id,
                op_type, payload, created_at, synced_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
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
        .await
        .context("append operation")?;

        Ok(())
    }

    pub async fn get_unsynced(&self, user_id: Uuid) -> Result<Vec<Operation>> {
        let rows: Vec<OperationRow> = sqlx::query_as(
            "SELECT * FROM operations WHERE user_id = ? AND synced_at IS NULL ORDER BY seq ASC",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("get unsynced operations")?;

        rows.into_iter()
            .map(|r| r.into_operation())
            .collect::<Result<Vec<_>>>()
    }

    pub async fn mark_synced(&self, op_ids: &[Uuid]) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await.context("begin transaction")?;

        for op_id in op_ids {
            sqlx::query("UPDATE operations SET synced_at = ? WHERE op_id = ?")
                .bind(&now)
                .bind(op_id.to_string())
                .execute(&mut *tx)
                .await
                .context("mark operation synced")?;
        }

        tx.commit().await.context("commit transaction")?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn get_next_seq(&self, user_id: Uuid, device_id: Uuid) -> Result<i64> {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT MAX(seq) FROM operations WHERE user_id = ? AND device_id = ?",
        )
        .bind(user_id.to_string())
        .bind(device_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("get next seq")?;

        Ok(result.map(|(max_seq,)| max_seq).unwrap_or(0) + 1)
    }

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
        let state_json =
            serde_json::to_string(&snapshot).context("serialize snapshot")?;

        sqlx::query(
            "INSERT INTO snapshots (user_id, device_id, snapshot_seq, state_json) VALUES (?, ?, ?, ?)",
        )
        .bind(user_id.to_string())
        .bind(device_id.to_string())
        .bind(snapshot_seq)
        .bind(&state_json)
        .execute(&self.pool)
        .await
        .context("create snapshot")?;

        Ok(())
    }

    pub async fn get_latest_snapshot(&self, user_id: Uuid) -> Result<Option<Snapshot>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT state_json FROM snapshots WHERE user_id = ? ORDER BY snapshot_seq DESC, id DESC LIMIT 1",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("get latest snapshot")?;

        match row {
            Some((json,)) => serde_json::from_str(&json)
                .context("deserialize snapshot")
                .map(Some),
            None => Ok(None),
        }
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
            op_id: Uuid::parse_str(&self.op_id).context("parse op_id")?,
            user_id: Uuid::parse_str(&self.user_id).context("parse user_id")?,
            device_id: Uuid::parse_str(&self.device_id).context("parse device_id")?,
            seq: self.seq,
            entity: serde_json::from_str(&self.entity).context("parse entity")?,
            entity_id: Uuid::parse_str(&self.entity_id).context("parse entity_id")?,
            op_type: serde_json::from_str(&self.op_type).context("parse op_type")?,
            payload: serde_json::from_str(&self.payload).context("parse payload")?,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)
                .context("parse created_at")?
                .with_timezone(&Utc),
            synced_at: match self.synced_at {
                Some(s) => Some(
                    DateTime::parse_from_rfc3339(&s)
                        .context("parse synced_at")?
                        .with_timezone(&Utc),
                ),
                None => None,
            },
        })
    }
}
