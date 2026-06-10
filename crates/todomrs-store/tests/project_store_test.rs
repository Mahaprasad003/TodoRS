use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use todomrs_core::domain::Project;
use todomrs_store::ProjectStore;
use uuid::Uuid;

fn make_project(user_id: Uuid, name: &str) -> Project {
    let now = Utc::now();
    Project {
        id: Uuid::new_v4(),
        user_id,
        name: name.to_string(),
        color: None,
        sort_order: 0,
        created_at: now,
        updated_at: now,
        archived_at: None,
    }
}

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

async fn setup() -> (SqlitePool, ProjectStore) {
    let pool = setup_pool().await.expect("test pool");
    let store = ProjectStore::new(pool.clone());
    (pool, store)
}

#[tokio::test]
async fn test_create_and_get_project() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let project = make_project(user_id, "My Project");
    store.create(&project).await?;

    let retrieved = store.get_by_id(project.id).await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "My Project");
    assert_eq!(retrieved.user_id, user_id);

    Ok(())
}

#[tokio::test]
async fn test_get_all_projects() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let p1 = make_project(user_id, "Alpha");
    let mut p2 = make_project(user_id, "Beta");
    p2.sort_order = 1;
    store.create(&p1).await?;
    store.create(&p2).await?;

    let projects = store.get_all(user_id).await?;
    assert_eq!(projects.len(), 2);
    // Ordered by sort_order
    assert_eq!(projects[0].name, "Alpha");

    Ok(())
}

#[tokio::test]
async fn test_update_project() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let mut project = make_project(user_id, "Original");
    store.create(&project).await?;

    project.name = "Updated".to_string();
    project.color = Some("#ff0000".to_string());
    project.updated_at = Utc::now();
    store.update(&project).await?;

    let retrieved = store.get_by_id(project.id).await?.unwrap();
    assert_eq!(retrieved.name, "Updated");
    assert_eq!(retrieved.color, Some("#ff0000".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_delete_project() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let project = make_project(user_id, "To delete");
    store.create(&project).await?;

    store.delete(project.id).await?;

    let retrieved = store.get_by_id(project.id).await?;
    assert!(retrieved.is_none());

    Ok(())
}
