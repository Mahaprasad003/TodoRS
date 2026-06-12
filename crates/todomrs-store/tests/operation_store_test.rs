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

#[tokio::test]
async fn test_get_next_seq_uses_remote_max_when_local_empty() -> Result<()> {
    let (_pool, mut store) = setup().await;
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    // No local ops exist, but remote_max_seq is set to 42
    store.set_remote_max_seq(Some(42));

    let seq = store.get_next_seq(user_id, device_id).await?;
    // Should return remote_max_seq + 1 (43) because local is empty
    assert_eq!(seq, 43, "should use remote_max_seq + 1 when local is empty");

    Ok(())
}

#[tokio::test]
async fn test_get_next_seq_prefers_local_when_local_exceeds_remote() -> Result<()> {
    let (_pool, mut store) = setup().await;
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    // Insert a local operation with seq=50
    let task = Task::new(user_id, "Test".to_string());
    let op = Operation::create_task(user_id, device_id, 50, &task);
    store.append(&op).await?;

    // remote_max_seq is lower (42)
    store.set_remote_max_seq(Some(42));

    let seq = store.get_next_seq(user_id, device_id).await?;
    // Should prefer local max (50) + 1 = 51
    assert_eq!(seq, 51, "should prefer local max when it exceeds remote");

    Ok(())
}

#[tokio::test]
async fn test_get_unsynced_is_user_scoped() -> Result<()> {
    let (_pool, store) = setup().await;
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    // Create ops for user_a
    let task_a = Task::new(user_a, "User A task".to_string());
    let op_a = Operation::create_task(user_a, device_id, 1, &task_a);
    store.append(&op_a).await?;

    // Create ops for user_b
    let task_b = Task::new(user_b, "User B task".to_string());
    let op_b = Operation::create_task(user_b, device_id, 1, &task_b);
    store.append(&op_b).await?;

    // user_a should only see user_a's unsynced ops
    let unsynced_a = store.get_unsynced(user_a).await?;
    assert_eq!(unsynced_a.len(), 1);
    assert_eq!(unsynced_a[0].user_id, user_a);

    // user_b should only see user_b's unsynced ops
    let unsynced_b = store.get_unsynced(user_b).await?;
    assert_eq!(unsynced_b.len(), 1);
    assert_eq!(unsynced_b[0].user_id, user_b);

    Ok(())
}
