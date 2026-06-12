# Implementation Plan: TodoRS Notification System

## What to implement

Read the plan at `/home/mp/Projects/TodoRS/plan.md` first. It's the full design doc. You're implementing it exactly as described.

## Scope

Two parallel tracks:

### PWA Notifications (~80 lines)

1. **Create `pwa/src/lib/notifications.ts`** — new file with:
   - `requestNotificationPermission()` — requests permission on first load
   - `checkNotifications(pendingTasks)` — the main logic (morning brief + due + overdue)
   - `sendNotification(body)` — wraps `new Notification()`
   - Helper: `localDate(d)` — returns YYYY-MM-DD in local timezone
   - Use existing `getMetadata`/`setMetadata` from `$lib/db/metadata` for persistence

2. **Modify `pwa/src/lib/sync/client.ts`** — after sync completes successfully, call `checkNotifications()` with pending tasks from `tasksStore`

3. **Modify `pwa/src/routes/+layout.svelte`** — call `requestNotificationPermission()` in the `onMount` block

### TUI Notifications (~80 lines)

4. **Add `notify-rust = "4"` to `crates/todomrs-tui/Cargo.toml`**

5. **Create `crates/todomrs-tui/src/notifications.rs`** — same logic as PWA but:
   - Uses `notify_rust::Notification` for desktop notifications
   - Uses SQLite `metadata` table for persistence
   - Include `mod notifications;` in `main.rs`

6. **Modify `crates/todomrs-tui/src/app.rs`** — call `check_notifications()` after sync in the `sync()` method

## Key design decisions (DO NOT change these)

- **State tracking:** `notified_tasks` stores `{ taskId: dueAtISO }` — if a task's due_at changes, the entry is stale and gets cleaned up (new notification opportunity)
- **Morning brief:** Fires once per day, shows first 2 task titles + "and N more"
- **Due notification:** "Time: title" — only for tasks with a time component, within 5min window
- **Overdue notification:** "title is overdue" — for tasks without time (date passed) or tasks with time (5min window missed)
- **No configuration.** No settings. No toggles. No retry on permission denied.
- **Cleanup:** Remove entries from `notified_tasks` when task is completed/deleted or when due_at changes

## What NOT to do

- Don't add any settings UI
- Don't add notification preferences
- Don't add snooze/dismiss actions
- Don't add badge counts
- Don't add server infrastructure (no VAPID, no Edge Functions)
- Don't modify the existing sync logic (just call checkNotifications after it)

## Testing

After implementation:
- `cargo test` must pass (all existing tests)
- `cargo build -p todomrs-tui` must succeed
- `cd pwa && npx svelte-check` must show 0 errors
- `cd pwa && npm run build` must succeed

## Commit

```
feat: add notification system for due/overdue tasks

- Morning brief on first sync of the day
- "Time: title" notification when task becomes due (within 5min window)
- "title is overdue" notification when task becomes overdue
- Browser Notification API for PWA, notify-rust for TUI
- No configuration, no server infrastructure
- State tracked in metadata store (IndexedDB/SQLite)
```
