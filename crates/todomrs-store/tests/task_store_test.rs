use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use todomrs_core::domain::{Priority, Task, TaskStatus};
use todomrs_store::TaskStore;
use uuid::Uuid;

/// In-memory DB setup that applies the project migration.
async fn setup_pool() -> Result<SqlitePool> {
    let pool = SqlitePool::connect("sqlite::memory:").await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;
    Ok(pool)
}

async fn seed_user(pool: &SqlitePool, user_id: Uuid) -> Result<()> {
    sqlx::query("INSERT INTO users (id, email) VALUES (?, ?)")
        .bind(user_id)
        .bind("test@example.com")
        .execute(pool)
        .await?;
    Ok(())
}

async fn setup() -> (SqlitePool, TaskStore) {
    let pool = setup_pool().await.expect("test pool");
    let store = TaskStore::new(pool.clone());
    (pool, store)
}

#[tokio::test]
async fn test_create_and_get_task() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let task = Task::new(user_id, "Test task".to_string());
    store.create(&task).await?;

    let retrieved = store.get_by_id(task.id).await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.title, "Test task");
    assert_eq!(retrieved.user_id, user_id);
    assert_eq!(retrieved.status, TaskStatus::Pending);
    assert_eq!(retrieved.priority, Priority::None);

    Ok(())
}

#[tokio::test]
async fn test_get_all_tasks() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let task1 = Task::new(user_id, "Task 1".to_string());
    let task2 = Task::new(user_id, "Task 2".to_string());
    store.create(&task1).await?;
    store.create(&task2).await?;

    let tasks = store.get_all(user_id).await?;
    assert_eq!(tasks.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_get_all_excludes_deleted() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let task = Task::new(user_id, "Will be soft-deleted".to_string());
    store.create(&task).await?;

    // Soft-delete by setting deleted_at
    let mut deleted = task.clone();
    deleted.deleted_at = Some(Utc::now());
    store.update(&deleted).await?;

    let tasks = store.get_all(user_id).await?;
    assert_eq!(tasks.len(), 0, "deleted tasks must be excluded from get_all");

    Ok(())
}

#[tokio::test]
async fn test_update_task() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let mut task = Task::new(user_id, "Original".to_string());
    store.create(&task).await?;

    task.title = "Updated".to_string();
    task.priority = Priority::High;
    task.updated_at = Utc::now();
    store.update(&task).await?;

    let retrieved = store.get_by_id(task.id).await?.unwrap();
    assert_eq!(retrieved.title, "Updated");
    assert_eq!(retrieved.priority, Priority::High);

    Ok(())
}

#[tokio::test]
async fn test_delete_task() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let task = Task::new(user_id, "To delete".to_string());
    store.create(&task).await?;

    store.hard_delete(task.id).await?;

    let retrieved = store.get_by_id(task.id).await?;
    assert!(retrieved.is_none());

    Ok(())
}

#[tokio::test]
async fn test_task_tags() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();
    let tag_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    // Create a tag so FK is satisfied
    sqlx::query("INSERT INTO tags (id, user_id, name) VALUES (?, ?, ?)")
        .bind(tag_id)
        .bind(user_id)
        .bind("test-tag")
        .execute(&pool)
        .await?;

    let mut task = Task::new(user_id, "Tagged task".to_string());
    task.tag_ids = vec![tag_id];
    store.create(&task).await?;

    // Verify tags loaded back
    let retrieved = store.get_by_id(task.id).await?.unwrap();
    assert_eq!(retrieved.tag_ids, vec![tag_id]);

    // Update: clear tags
    let mut updated = task.clone();
    updated.tag_ids = Vec::new();
    store.update(&updated).await?;

    let retrieved2 = store.get_by_id(task.id).await?.unwrap();
    assert!(retrieved2.tag_ids.is_empty());

    Ok(())
}
