use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use todomrs_core::domain::Tag;
use todomrs_store::TagStore;
use uuid::Uuid;

fn make_tag(user_id: Uuid, name: &str) -> Tag {
    let now = Utc::now();
    Tag {
        id: Uuid::new_v4(),
        user_id,
        name: name.to_string(),
        color: None,
        created_at: now,
        updated_at: now,
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

async fn setup() -> (SqlitePool, TagStore) {
    let pool = setup_pool().await.expect("test pool");
    let store = TagStore::new(pool.clone());
    (pool, store)
}

#[tokio::test]
async fn test_create_and_get_tag() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let tag = make_tag(user_id, "urgent");
    store.create(&tag).await?;

    let retrieved = store.get_by_id(tag.id).await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "urgent");
    assert_eq!(retrieved.user_id, user_id);

    Ok(())
}

#[tokio::test]
async fn test_get_all_tags() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let t1 = make_tag(user_id, "alpha");
    let t2 = make_tag(user_id, "beta");
    store.create(&t1).await?;
    store.create(&t2).await?;

    let tags = store.get_all(user_id).await?;
    assert_eq!(tags.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_update_tag() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let mut tag = make_tag(user_id, "original");
    store.create(&tag).await?;

    tag.name = "renamed".to_string();
    tag.color = Some("#00ff00".to_string());
    tag.updated_at = Utc::now();
    store.update(&tag).await?;

    let retrieved = store.get_by_id(tag.id).await?.unwrap();
    assert_eq!(retrieved.name, "renamed");
    assert_eq!(retrieved.color, Some("#00ff00".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_delete_tag() -> Result<()> {
    let (pool, store) = setup().await;
    let user_id = Uuid::new_v4();

    seed_user(&pool, user_id).await?;

    let tag = make_tag(user_id, "to-delete");
    store.create(&tag).await?;

    store.hard_delete(tag.id).await?;

    let retrieved = store.get_by_id(tag.id).await?;
    assert!(retrieved.is_none());

    Ok(())
}
