use anyhow::Result;
use chrono::{Local, Timelike, Utc};
use notify_rust::Notification;
use sqlx::SqlitePool;
use std::collections::HashMap;
use todomrs_core::domain::Task;

/// Check all three notification moments after a sync completes.
///
/// Uses the metadata table in SQLite for state tracking, same keys as the PWA.
///
/// # Per-account safety
///
/// The metadata keys `last_daily_notify` and `notified_tasks` are read from
/// and written to the per-account database. Since each authenticated account
/// opens a different SQLite file, notification state cannot bleed across
/// accounts. This was previously a cross-account concern when all accounts
/// shared one global database.
pub async fn check_notifications(pool: &SqlitePool, tasks: &[Task]) -> Result<()> {
    let pending: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.status == todomrs_core::domain::TaskStatus::Pending && t.deleted_at.is_none())
        .collect();

    let now = Utc::now();
    let today = local_date_today();

    // ── 1. Morning Brief ──
    let last_brief = get_metadata(pool, "last_daily_notify").await?;
    if last_brief.as_deref() != Some(&today) {
        let today_tasks: Vec<&&Task> = pending
            .iter()
            .filter(|t| {
                t.due_at
                    .map(|d| format_local_date(d))
                    .as_deref()
                    == Some(&today)
            })
            .collect();

        if !today_tasks.is_empty() {
            let titles: Vec<&str> = today_tasks.iter().take(2).map(|t| t.title.as_str()).collect();
            let rest = today_tasks.len() - titles.len();
            let mut text = format!("Today: {}", titles.join(", "));
            if rest == 1 {
                text.push_str(" and 1 more");
            } else if rest > 1 {
                text.push_str(&format!(" and {} more", rest));
            }
            let _ = send_notification(&text);
        }
        set_metadata(pool, "last_daily_notify", &today).await?;
    }

    // ── 2 & 3. Task due / overdue ──
    let notified_json = get_metadata(pool, "notified_tasks").await?;
    let notified: HashMap<String, String> = notified_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let mut cleaned: HashMap<String, String> = HashMap::new();

    for task in &pending {
        let task_id = task.id.to_string();
        let stored_due = notified.get(&task_id);

        if let Some(stored) = stored_due {
            // Previously notified. Keep only if the due_at hasn't changed.
            let current_due = task.due_at.map(|d| d.to_rfc3339());
            if current_due.as_deref() == Some(stored) {
                cleaned.insert(task_id, stored.clone());
            }
            // If due_at changed, drop the entry. New due date = fresh notification.
            continue;
        }

        let due_at = match task.due_at {
            Some(d) => d,
            None => continue,
        };

        let has_time = due_at.hour() != 0 || due_at.minute() != 0;

        if has_time {
            let five_min_later = due_at + chrono::Duration::minutes(5);
            if now >= due_at && now < five_min_later {
                // Moment 2: Task becomes due
                let _ = send_notification(&format!("Time: {}", task.title));
                cleaned.insert(task_id, due_at.to_rfc3339());
            } else if now >= five_min_later {
                // Moment 3: Task overdue
                let _ = send_notification(&format!("{} is overdue", task.title));
                cleaned.insert(task_id, due_at.to_rfc3339());
            }
        } else {
            // No time component — midnight
            let due_date = format_local_date(due_at);
            if due_date < today {
                // Moment 3: Date passed
                let _ = send_notification(&format!("{} is overdue", task.title));
                cleaned.insert(task_id, due_at.to_rfc3339());
            }
            // Tasks due today without a time: covered by morning brief
        }
    }

    let cleaned_json = serde_json::to_string(&cleaned)?;
    set_metadata(pool, "notified_tasks", &cleaned_json).await?;

    Ok(())
}

fn send_notification(body: &str) -> Result<()> {
    Notification::new()
        .summary("TodoRS")
        .body(body)
        .show()?;
    Ok(())
}

/// Return today's local date as YYYY-MM-DD.
fn local_date_today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Format a datetime's local date as YYYY-MM-DD.
fn format_local_date(dt: chrono::DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%Y-%m-%d").to_string()
}

// ── Metadata helpers (reuse the existing metadata table) ──

async fn get_metadata(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM metadata WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

async fn set_metadata(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query("INSERT OR REPLACE INTO metadata (key, value) VALUES (?, ?)")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}
