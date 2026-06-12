# TodoRS Notification System

## The Philosophy

Notifications should feel like a helpful assistant, not a nagging boss. The user should never see the same notification twice, and should never be annoyed. This is a personal app for one user — no configuration needed, no toggles, no settings screen. The app just does the right thing.

## Three Notification Moments

We support exactly three notification moments. Each fires at most once per trigger. Once fired, it never fires again for the same state.

### 1. Morning Brief (First Sync of the Day)

**Trigger:** The current date (local timezone) is different from `last_daily_notify` stored in metadata.

**What it notifies:** All pending tasks where `date(due_at) == today`.

**Notification text:**
- 1 task: `"Today: buy milk"`
- 2-3 tasks: `"Today: buy milk, pay bills, call mom"`
- 4+ tasks: `"Today: buy milk, pay bills, and 2 more"` (show first 2 + count)
- 0 tasks: No notification (don't notify about an empty day)

**Once-ness:** Store `last_daily_notify = today` (YYYY-MM-DD string) in metadata. On next sync, `today == last_daily_notify` so the brief doesn't fire.

**Why not include overdue tasks from previous days?** The user already knows about them — they're sitting in the task list with a red indicator. The morning brief is a preview of what's coming today, not a rehash of what they've already ignored.

**What if the user hasn't opened the app in 3 days?** On first sync of the new day, the morning brief fires for TODAY's tasks only. It does NOT fire briefs for the 2 missed days. One brief per day, max. The user sees today's slate, and the overdue detection (see below) handles any tasks that slipped.

---

### 2. Task Becomes Due (Sync Catches the Exact Moment)

**Trigger:** The task has a **time component** in `due_at` (not midnight), AND `now >= due_at`, AND `now < due_at + 5 minutes`, AND the task's ID is not in `notified_tasks`.

**Why the 5-minute window?** The sync runs every 30 seconds. A task due at 3:00:00pm might be caught at 3:00:00, 3:00:30, or 3:01:00. If the user opens the app at 3:02pm, the sync at 3:02:00 would catch it (3:02 < 3:05). If the user opens at 3:06pm, the 5-minute window has passed, and the task falls through to the overdue detection (moment #3) instead.

**Notification text:** `"Time: buy milk"`

**Once-ness:** Add `notified_tasks[task.id] = "due"` to metadata. On next sync, the task is in `notified_tasks` so it's skipped.

**Why "Time:" and not "due"?** The word "Time" conveys urgency without being alarmist. It says "this is the moment you said it would be." It's a gentle nudge, not a command.

---

### 3. Task Becomes Overdue (First Sync That Sees It Overdue)

**Trigger:** Two sub-cases:

**Case A — Task has NO time component (midnight):**
`date(due_at) < today` (the date has passed) AND the task's ID is not in `notified_tasks`.

**Case B — Task HAS a time component, but the 5-minute "due" window was missed:**
`now >= due_at + 5 minutes` AND the task's ID is not in `notified_tasks`.

**Notification text:** `"buy milk is overdue"`

**Once-ness:** Add `notified_tasks[task.id] = "overdue"` to metadata. On next sync, the task is in `notified_tasks` so it's skipped.

**What about tasks that are 3 days overdue?** We notify ONCE when we first detect them as overdue. We do NOT re-notify every 30 seconds. The task just sits in the list with a red indicator. If the user completes it, it's gone. If they change the due date, it's a new state and the notification logic resets for that task.

---

## State Tracking

All notification state lives in the existing `metadata` store (key-value in IndexedDB for PWA, SQLite for TUI).

### Keys

```
last_daily_notify: "2026-06-13"          // YYYY-MM-DD of last morning brief
notified_tasks: {                         // task ID → notification reason
  "uuid-1": "due",
  "uuid-2": "overdue",
}
```

### Size

The `notified_tasks` map grows over time. We need a cleanup strategy:

- **On morning brief:** Remove entries older than 7 days (the task is either completed, or the user has had a week to deal with it and re-notifying won't help).
- **On task completion:** Remove the entry from `notified_tasks` (the task is gone, no need to track it).

This keeps the map bounded to ~30-50 entries max for a heavy user.

---

## The Exact Logic

```
function checkNotifications(pendingTasks):
  if permission !== 'granted': return
  
  now = new Date()
  today = localDate(now)  // YYYY-MM-DD
  yesterday = today - 1 day
  
  // ── 1. Morning Brief ──
  lastBrief = metadata.get("last_daily_notify")
  if today != lastBrief:
    todayTasks = pendingTasks.filter(t => date(t.due_at) == today)
    if todayTasks.length > 0:
      titles = todayTasks.slice(0, 2).map(t => t.title)
      rest = todayTasks.length - titles.length
      text = "Today: " + titles.join(", ")
      if rest > 0:
        text += rest == 1 ? " and 1 more" : ` and ${rest} more`
      notify(text)
    metadata.set("last_daily_notify", today)
  
  // ── 2 & 3. Task Due / Overdue ──
  notified = metadata.get("notified_tasks") || {}
  cleaned = {}  // for cleanup
  
  for task in pendingTasks:
    if task.id in notified:
      // Keep entry if less than 7 days old (cleanup)
      // (We don't store timestamps, just the reason. Cleanup happens
      //  by removing entries for completed/deleted tasks, which we
      //  handle by rebuilding the map from current pending tasks.)
      cleaned[task.id] = notified[task.id]
      continue
    
    dueAt = new Date(task.due_at)
    hasTime = dueAt is not midnight
    
    if hasTime:
      // Task has a specific time
      if now >= dueAt && now < dueAt + 5min:
        // Moment #2: Task becomes due
        notify("Time: " + task.title)
        cleaned[task.id] = "due"
      else if now >= dueAt + 5min:
        // Moment #3: Task became overdue (missed the due window)
        notify(task.title + " is overdue")
        cleaned[task.id] = "overdue"
      else:
        // Task is not yet due, no notification
        pass
    else:
      // Task has no specific time (midnight)
      if date(dueAt) < today:
        // Moment #3: Task is overdue
        notify(task.title + " is overdue")
        cleaned[task.id] = "overdue"
      else:
        // Task is due today but has no time — covered by morning brief
        pass
  
  metadata.set("notified_tasks", cleaned)
```

### Cleanup Strategy

Instead of storing timestamps for cleanup, we rebuild the `notified_tasks` map on each sync:

1. Start with the existing `notified_tasks` map
2. For each entry, check if the task still exists in `pendingTasks`
3. If the task is no longer pending (completed, deleted, or due date changed), drop the entry
4. Save the cleaned map

This way, the map only ever contains entries for tasks that are still pending. Completed tasks are automatically cleaned up.

---

## PWA Implementation

### Files

| File | Changes | Lines |
|------|---------|-------|
| `pwa/src/lib/notifications.ts` | New file. Notification logic + IndexedDB helpers | ~100 |
| `pwa/src/lib/sync/client.ts` | Call `checkNotifications()` after sync completes | ~5 |
| `pwa/src/routes/+layout.svelte` | Request notification permission on mount | ~10 |

### `pwa/src/lib/notifications.ts`

```typescript
import { getMetadata, setMetadata } from '$lib/db/metadata'

const META_DAILY = 'last_daily_notify'
const META_NOTIFIED = 'notified_tasks'

export async function requestNotificationPermission(): Promise<boolean> {
  if (!('Notification' in window)) return false
  if (Notification.permission === 'granted') return true
  if (Notification.permission === 'denied') return false
  const result = await Notification.requestPermission()
  return result === 'granted'
}

export async function checkNotifications(pendingTasks: TaskRecord[]): Promise<void> {
  if (!('Notification' in window) || Notification.permission !== 'granted') return

  const now = new Date()
  const today = localDate(now)

  // 1. Morning brief
  const lastBrief = await getMetadata(META_DAILY)
  if (today !== lastBrief) {
    const todayTasks = pendingTasks.filter(t => t.due_at && localDate(new Date(t.due_at)) === today)
    if (todayTasks.length > 0) {
      const titles = todayTasks.slice(0, 2).map(t => t.title)
      const rest = todayTasks.length - titles.length
      let text = 'Today: ' + titles.join(', ')
      if (rest === 1) text += ' and 1 more'
      else if (rest > 1) text += ` and ${rest} more`
      sendNotification(text)
    }
    await setMetadata(META_DAILY, today)
  }

  // 2 & 3. Task due / overdue
  const notified: Record<string, string> = JSON.parse(await getMetadata(META_NOTIFIED) || '{}')
  const cleaned: Record<string, string> = {}

  for (const task of pendingTasks) {
    // Cleanup: keep only entries for still-pending tasks
    if (task.id in notified) {
      cleaned[task.id] = notified[task.id]
      continue
    }

    if (!task.due_at) continue
    const dueAt = new Date(task.due_at)
    const hasTime = dueAt.getUTCHours() !== 0 || dueAt.getUTCMinutes() !== 0

    if (hasTime) {
      const fiveMinAfter = new Date(dueAt.getTime() + 5 * 60 * 1000)
      if (now >= dueAt && now < fiveMinAfter) {
        // Moment 2: Task becomes due
        sendNotification('Time: ' + task.title)
        cleaned[task.id] = 'due'
      } else if (now >= fiveMinAfter) {
        // Moment 3: Task overdue (missed due window)
        sendNotification(task.title + ' is overdue')
        cleaned[task.id] = 'overdue'
      }
    } else {
      // No time component — check if date has passed
      if (localDate(dueAt) < today) {
        sendNotification(task.title + ' is overdue')
        cleaned[task.id] = 'overdue'
      }
    }
  }

  await setMetadata(META_NOTIFIED, JSON.stringify(cleaned))
}

function sendNotification(body: string): void {
  new Notification('TodoRS', {
    body,
    icon: '/icon-192.png',
    badge: '/icon-192.png',
    tag: 'todors-' + Date.now(),  // unique tag prevents stacking
  })
}

function localDate(d: Date): string {
  return d.toLocaleDateString('en-CA')  // YYYY-MM-DD in local timezone
}
```

### Integration in `sync/client.ts`

After the sync completes successfully (inside the `try` block, after updating sync state), call:

```typescript
import { checkNotifications } from '$lib/notifications'
import { tasksStore } from '$lib/stores/tasks'

// ... after sync state update ...
const pending = get(tasksStore).filter(t => t.status === 'pending' && !t.deleted_at)
await checkNotifications(pending)
```

### Permission Request in `+layout.svelte`

In the `onMount` block, after `initAuth()`:

```typescript
import { requestNotificationPermission } from '$lib/notifications'

// In onMount:
requestNotificationPermission()
```

This will prompt the user for notification permission the first time they load the app. If they deny, notifications just won't work (no error, no retry). If they grant, they'll get notifications from then on.

---

## TUI Implementation

### Files

| File | Changes | Lines |
|------|---------|-------|
| `crates/todomrs-tui/src/notifications.rs` | New file. Notification logic + SQLite helpers | ~80 |
| `crates/todomrs-tui/src/app.rs` | Call `check_notifications()` after sync | ~5 |
| `crates/todomrs-tui/src/main.rs` | Add `notify-rust` dependency | — |
| `crates/todomrs-tui/Cargo.toml` | Add `notify-rust` | 1 |

### Approach

The TUI uses the same logic as the PWA but with two differences:

1. **Notification delivery:** Instead of `new Notification()`, we use `notify-rust` for native desktop notifications:
   ```rust
   Notification::new()
       .summary("TodoRS")
       .body(&text)
       .show()
   ```

2. **State storage:** Instead of IndexedDB metadata, we use SQLite. The `metadata` table already exists (key-value store). We use the same keys: `last_daily_notify` and `notified_tasks`.

### `crates/todomrs-tui/src/notifications.rs`

```rust
use anyhow::Result;
use chrono::{Local, Utc};
use notify_rust::Notification;
use sqlx::SqlitePool;
use std::collections::HashMap;
use todomrs_core::domain::{Task, TaskStatus};

pub async fn check_notifications(pool: &SqlitePool, tasks: &[Task]) -> Result<()> {
    let pending: Vec<&Task> = tasks.iter()
        .filter(|t| t.status == TaskStatus::Pending && t.deleted_at.is_none())
        .collect();

    let now = Utc::now();
    let today = Local::now().format("%Y-%m-%d").to_string();

    // 1. Morning brief
    let last_brief = get_metadata(pool, "last_daily_notify").await?;
    if last_brief.as_deref() != Some(&today) {
        let today_tasks: Vec<&Task> = pending.iter()
            .filter(|t| t.due_at.map(|d| d.with_timezone(&Local).format("%Y-%m-%d").to_string()) == Some(today.clone()))
            .copied()
            .collect();
        
        if !today_tasks.is_empty() {
            let titles: Vec<&str> = today_tasks.iter().take(2).map(|t| t.title.as_str()).collect();
            let rest = today_tasks.len() - titles.len();
            let mut text = format!("Today: {}", titles.join(", "));
            if rest == 1 { text.push_str(" and 1 more"); }
            else if rest > 1 { text.push_str(&format!(" and {} more", rest)); }
            send_notification(&text)?;
        }
        set_metadata(pool, "last_daily_notify", &today).await?;
    }

    // 2 & 3. Task due / overdue
    let notified_json = get_metadata(pool, "notified_tasks").await?;
    let notified: HashMap<String, String> = notified_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut cleaned: HashMap<String, String> = HashMap::new();

    for task in &pending {
        if notified.contains_key(&task.id.to_string()) {
            cleaned.insert(task.id.to_string(), notified[&task.id.to_string()].clone());
            continue;
        }

        let due_at = match task.due_at {
            Some(d) => d,
            None => continue,
        };

        let local_due = due_at.with_timezone(&Local);
        let is_midnight = local_due.hour() == 0 && local_due.minute() == 0;

        if !is_midnight {
            let five_min_later = due_at + chrono::Duration::minutes(5);
            if now >= due_at && now < five_min_later {
                send_notification(&format!("Time: {}", task.title))?;
                cleaned.insert(task.id.to_string(), "due".to_string());
            } else if now >= five_min_later {
                send_notification(&format!("{} is overdue", task.title))?;
                cleaned.insert(task.id.to_string(), "overdue".to_string());
            }
        } else {
            let due_date = local_due.format("%Y-%m-%d").to_string();
            if due_date < today {
                send_notification(&format!("{} is overdue", task.title))?;
                cleaned.insert(task.id.to_string(), "overdue".to_string());
            }
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

// Metadata helpers (reuse existing metadata table)
async fn get_metadata(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM metadata WHERE key = ?"
    )
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
```

### Integration in `app.rs`

In the `sync()` method, after the sync state is updated and before the status message is set:

```rust
// Check notifications
if let Err(e) = notifications::check_notifications(self.task_store.pool(), &self.tasks).await {
    eprintln!("Notification check failed: {}", e);
}
```

---

## What We Chose Not To Do (and Why)

### No "snooze" or "dismiss" action
The user can complete or reschedule the task. That's the natural way to make a notification go away. Adding snooze buttons adds complexity without proportional value for a personal app.

### No notification preferences
No "quiet hours," no "only notify for high priority," no toggles. The three moments we chose are the right moments. If the user wants to change the behavior, they edit the code.

### No re-notification for overdue tasks
Once we say "buy milk is overdue," we don't say it again. The task stays in the list. If the user wants to be reminded, they reschedule it (which creates a new due date and a new notification opportunity).

### No notification for tasks due today (without time)
Tasks due today without a specific time are covered by the morning brief. We don't individually notify "buy milk is due" for these — that would be redundant with the brief.

### No badge count or app icon changes
We don't modify the PWA's app icon or add badge numbers. The notification itself is the alert. Adding badges adds complexity and is platform-dependent.

---

## Edge Cases

### User opens app for the first time after 3 days
- Morning brief fires for TODAY's tasks only (not the 2 missed days)
- Overdue detection fires for each task that became overdue while the app was closed
- Total notifications: 1 morning brief + N overdue notifications (one per overdue task)
- This is not spam — it's a catch-up. The user gets one message per overdue task, not repeated messages.

### User completes a task that was notified as overdue
- The task is no longer pending
- On next sync, the cleanup removes it from `notified_tasks`
- No further notifications for that task

### User changes a task's due date
- The task's ID is still in `notified_tasks` with the old reason
- But the due date check will fail (the new due date is in the future)
- When the new due date arrives, the notification fires again
- Wait — the task is still in `notified_tasks`! We need to handle this.
- Fix: In the cleanup step, if a task's due date has changed since the notification was sent, remove it from `notified_tasks`. We'd need to store the due date alongside the notification reason.
- Simpler fix: Don't store the reason, just store the due_at that was notified. If the task's current due_at differs from the stored one, the notification is stale and we remove it.

Let me update the state tracking:

```
notified_tasks: {
  "uuid-1": "2026-06-13T15:00:00.000Z",  // the due_at we notified about
  "uuid-2": "2026-06-12T00:00:00.000Z",
}
```

In the cleanup:
```
for (taskId, notifiedDueAt) in notified_tasks:
  task = pendingTasks.find(t => t.id == taskId)
  if !task:
    // Task is gone (completed/deleted), drop entry
    continue
  if task.due_at != notifiedDueAt:
    // Due date changed, notification is stale, drop entry
    // The new due date will trigger a new notification when it arrives
    continue
  // Task still pending with same due date, keep entry
  cleaned[taskId] = notifiedDueAt
```

This is cleaner and handles the reschedule case naturally.

### Multiple tasks become overdue at the same time
- Each gets its own notification: "buy milk is overdue", "pay bills is overdue"
- We don't batch overdue notifications (too complex, and each task deserves its own call to action)
- The browser may group them visually, which is fine

### Notification permission denied
- `checkNotifications()` returns early without error
- No fallback, no retry, no error message
- The user can re-enable in browser settings if they change their mind

---

## Testing

### PWA
1. Open PWA, grant notification permission
2. Create a task due "tomorrow 3pm"
3. Wait until 3pm (or mock the time)
4. Verify notification: "Time: [title]"
5. Wait 30 seconds, verify no duplicate notification
6. Create a task due "yesterday"
7. Verify notification: "[title] is overdue"
8. Reload page (new day), verify morning brief includes today's tasks
9. Complete a notified task, verify it's removed from `notified_tasks`
10. Change a task's due date, verify new notification fires at new time

### TUI
1. Launch TUI, verify notification permission works (notify-rust doesn't need permission on Linux)
2. Same test cases as PWA
3. Verify notifications appear as native desktop notifications

---

## Summary

| Moment | Trigger | Text | Once-ness |
|--------|---------|------|-----------|
| Morning brief | First sync of new day | "Today: task1, task2" | `last_daily_notify` date |
| Task becomes due | Sync catches `now >= due_at` within 5min | "Time: title" | `notified_tasks[id]` |
| Task becomes overdue | Date passed or 5min window missed | "title is overdue" | `notified_tasks[id]` |

Total: ~80 lines PWA, ~80 lines TUI, ~5 lines integration each. No infrastructure. No configuration. Just a helpful assistant that does the right thing.
