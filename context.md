Here are my complete findings:

---

# Code Context

## Files Retrieved

1. **`crates/todomrs-tui/src/app.rs`** — All of lines 1–1163 + 1164–1532 (read in two parts). Core sync timing logic.
2. **`crates/todomrs-tui/src/main.rs`** — Lines 1–168. Event loop, initial sync, `maybe_auto_sync()` call site.

## Key Code

### `App::new()` — construction
- **Line 127:** `last_sync_attempt: Instant::now(),` — initialized to wall clock at construction.
- **Line 128:** `sync_debounce_requested: false,` — initialized to false.

### `request_sync_after_mutation()` — line 1000–1002
```rust
pub fn request_sync_after_mutation(&mut self) {
    self.sync_debounce_requested = true;
}
```
Only sets a boolean flag. Does **not** record the time of the mutation.

### `maybe_auto_sync()` — lines 986–996
```rust
pub async fn maybe_auto_sync(&mut self) -> bool {
    let should_periodic = self.last_sync_attempt.elapsed() >= std::time::Duration::from_secs(30);
    let should_debounce = self.sync_debounce_requested
        && self.last_sync_attempt.elapsed() >= std::time::Duration::from_secs(10);

    if (should_periodic || should_debounce) && self.sync_client.is_some() {
        self.sync_debounce_requested = false;
        self.sync().await.ok();
        return true;
    }
    false
}
```
Key: both `should_periodic` (line 987) and `should_debounce` (lines 988–989) compare against `self.last_sync_attempt`, **not** against when the mutation happened.

### `sync()` — lines 1005–1072+
- **Line 1015:** `self.last_sync_attempt = Instant::now();` — resets the timer at the start of every sync.

### Event loop — `main.rs` lines 129–154
- **Line 131:** `app.sync().await?;` — initial sync right after construction.
- **Line 138:** `app.maybe_auto_sync().await;` — called every ~100ms loop iteration.
- **Line 152:** `app.sync().await.ok();` — exit sync on quit.

### Manual sync trigger — `app.rs` line 278–280
```rust
KeyCode::Char('s') | KeyCode::Char('S') => {
    self.sync().await?;
}
```
Manual `s` key calls `sync()` directly.

### All call sites of `request_sync_after_mutation()` — in `app.rs`:
| Line | Method |
|------|--------|
| 376 | `handle_event` → `add_project` path (Enter in Editing mode on Projects view) |
| 731 | `create_task_from_input` |
| 799 | `update_task_from_input` |
| 862 | `toggle_complete` |
| 901 | `delete_task` |
| 942 | `clear_completed` |
| 980 | `spawn_next_recurrence` |
| 1447 | `delete_project` |

Every mutation path calls `request_sync_after_mutation()`, but none of them record a timestamp or reset `last_sync_attempt`.

## Architecture

The sync-timing state machine uses two independent variables:
- **`last_sync_attempt`** (line 68, `Instant`) — when the last sync started/was attempted.
- **`sync_debounce_requested`** (line 70, `bool`) — whether a mutation happened since the last sync.

The debounce logic in `maybe_auto_sync()` (lines 986–996) checks: *"has 10 seconds elapsed since `last_sync_attempt` AND has a mutation been requested?"* This means the 10-second countdown starts from the **last sync**, not from the **last mutation**.

The flow:
```
Mutation occurs
   → request_sync_after_mutation() sets sync_debounce_requested = true
   → Next event loop iteration (≤100ms later):
       maybe_auto_sync() checks last_sync_attempt.elapsed() >= 10s
       If ≥10s since last sync → sync fires immediately
       If <10s since last sync → wait until 10s from last sync, then fire
```

## Start Here

Open **`/home/mp/Projects/TodoRS/crates/todomrs-tui/src/app.rs`** lines 984–1002 (the `maybe_auto_sync` and `request_sync_after_mutation` methods). The bug is that there is no mutation-timestamp field. The debounce uses `last_sync_attempt` as its time reference, so the "10 second debounce" is effectively "wait at least 10 seconds between syncs" rather than "wait 10 seconds after a mutation."

## Answers to Specific Questions

### Q1: Could `sync()` itself be setting `last_sync_attempt` to `Instant::now()` which then causes `maybe_auto_sync()` to see a fresh timer and fire early?

**No** — that's actually protective. `sync()` at **line 1015** resets `last_sync_attempt = Instant::now()`, so after sync completes, the timer starts fresh and the 10-second debounce window begins anew. This prevents rapid-fire syncs. The problem is that the debounce timer is *not anchored to the mutation time*.

### Q2: Could the 30s periodic sync be firing early because `last_sync_attempt` is initialized to `Instant::now()` at construction and the initial sync doesn't update it?

**No** — the initial sync at `main.rs` **line 131** calls `app.sync().await?`, which enters `sync()` at line 1005 and hits `self.last_sync_attempt = Instant::now()` at line 1015, overriding the construction value. So `last_sync_attempt` always reflects the actual last sync time (or a very recent time from construction if sync was skipped). The 30s periodic timer works correctly.

### Q3: Is there any other path that triggers sync immediately after a mutation?

**Yes — the actual root cause.** Here is the exact bug:

**Lines 988–989** check `self.last_sync_attempt.elapsed() >= std::time::Duration::from_secs(10)`. There is **no separate field** recording when the mutation occurred. So if the user has been idle for ≥10 seconds since the last sync, then makes a mutation:

1. `last_sync_attempt.elapsed()` is already ≥10s.
2. `sync_debounce_requested` is now `true`.
3. On the very next event loop iteration (~100ms), `maybe_auto_sync()` sees both conditions true.
4. Sync fires immediately (within ~100ms of the mutation, not 10 seconds later).

The fix would require adding a new field (e.g., `last_mutation_at: Instant`) that `request_sync_after_mutation()` sets to `Instant::now()`, then changing the debounce check at line 989 to compare against `last_mutation_at` instead of `last_sync_attempt`.