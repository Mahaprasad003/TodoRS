use anyhow::Result;
use sqlx::SqlitePool;
use todomrs_core::domain::Task;
use todomrs_store::OperationStore;
use todomrs_sync::operations::Operation;
use uuid::Uuid;

async fn setup_pool() -> Result<SqlitePool> {
    let pool = SqlitePool::connect("sqlite::memory:").await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;
    Ok(pool)
}

async fn setup() -> (SqlitePool, OperationStore) {
    let pool = setup_pool().await.expect("test pool");
    let store = OperationStore::new(pool.clone());
    (pool, store)
}

#[tokio::test]
async fn test_append_and_get_unsynced() -> Result<()> {
    let (_pool, store) = setup().await;
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    let task = Task::new(user_id, "Test".to_string());
    let op = Operation::create_task(user_id, device_id, 1, &task);

    store.append(&op).await?;

    let unsynced = store.get_unsynced(user_id).await?;
    assert_eq!(unsynced.len(), 1);
    assert_eq!(unsynced[0].op_id, op.op_id);

    Ok(())
}

#[tokio::test]
async fn test_mark_synced() -> Result<()> {
    let (_pool, store) = setup().await;
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    let task = Task::new(user_id, "Test".to_string());
    let op = Operation::create_task(user_id, device_id, 1, &task);

    store.append(&op).await?;
    store.mark_synced(&[op.op_id]).await?;

    let unsynced = store.get_unsynced(user_id).await?;
    assert_eq!(unsynced.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_get_next_seq() -> Result<()> {
    let (_pool, store) = setup().await;
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    let seq1 = store.get_next_seq(user_id, device_id).await?;
    assert_eq!(seq1, 1);

    let task = Task::new(user_id, "Test".to_string());
    let op = Operation::create_task(user_id, device_id, 1, &task);
    store.append(&op).await?;

    let seq2 = store.get_next_seq(user_id, device_id).await?;
    assert_eq!(seq2, 2);

    Ok(())
}

#[tokio::test]
async fn test_snapshot_create_and_retrieve() -> Result<()> {
    let (_pool, store) = setup().await;
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    let task = Task::new(user_id, "Snapshot task".to_string());
    store
        .create_snapshot(user_id, device_id, 1, vec![task], vec![], vec![])
        .await?;

    let snapshot = store.get_latest_snapshot(user_id).await?;
    assert!(snapshot.is_some());
    let snapshot = snapshot.unwrap();
    assert_eq!(snapshot.tasks.len(), 1);
    assert_eq!(snapshot.tasks[0].title, "Snapshot task");

    Ok(())
}
