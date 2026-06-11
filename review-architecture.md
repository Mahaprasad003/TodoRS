# Phase 8 Architecture & Maintainability Review

**Reviewed:** 2026-06-11
**Scope:** `plan.md` (Phase 8 TUI Sync Client Integration) against actual codebase

---

## 1. `sync_client: Option<SyncClient>` — Correct for Phase 8

**Severity:** Correct

- `SyncClient` is not `Clone` (it owns a `reqwest::Client` and `String` fields). The `App` struct is also a single-owner central state object. `Option<SyncClient>` is the idiomatic pattern for optional single ownership in a non-`Clone` struct.
- The `App::sync()` method takes `&mut self`, so `match &self.sync_client` borrows `&self` while `&mut self` is held, which is safe and legal.
- The plan correctly notes that `SyncClient` is "not Clone — can't easily share between threads" and that "Only used from main async task — no concurrency issue."
- **No blocker.** A trait-based abstraction (e.g., `Box<dyn SyncClient>`) would be over-engineering at this phase. `Option` is the right choice.

---

## 2. `last_synced_seq` In-Memory Only — Acceptable for Phase 8, but Underlying Protocol Is Broken

**Severity:** Blocker

### What the plan says
> "Keep it in-memory. The operation application is idempotent, so re-downloading is safe. Optimize in a later phase by storing `last_synced_seq` in the `sync_state` SQLite table."

### The real problem
The plan **does** store `last_synced_seq` in the `App` struct and passes it to `get_operations()` on every sync. The server endpoint (`supabase/functions/get-operations/index.ts`) queries:

```typescript
.from('operations')
.select('*')
.eq('user_id', user.id)
.gt('seq', parseInt(sinceSeq))   // ← global seq filter, no device_id
```

The backend schema (`backend/migrations/001_init.sql`) defines:

```sql
UNIQUE(user_id, device_id, seq)
```

**This means `seq` is per-device, not global.**

### Why the plan's logic is broken

The plan updates `last_synced_seq` with the **maximum** seq seen across **all** downloaded operations:

```rust
for op in &remote_ops {
    if op.device_id != self.device_id {
        if let Err(e) = self.apply_remote_operation(op).await { ... }
    }
    if op.seq > self.last_synced_seq {
        self.last_synced_seq = op.seq;  // ← BUG: tracks across all devices
    }
}
```

**Scenario demonstrating data loss:**

| Step | Device A | Device B | Server `last_synced_seq` (plan) | Result of `get_operations(seq)` |
|------|----------|----------|--------------------------------|--------------------------------|
| 1 | Creates op seq=1..50 | Creates op seq=1..5 | — | — |
| 2 | Syncs → gets A:1-50, B:1-5 | — | 50 | — |
| 3 | — | Creates op seq=6 | — | — |
| 4 | Syncs with `get_operations(50)` | — | — | **Misses B:6** because `seq=6 ≤ 50` |

**Device B's operations are permanently lost after Device A syncs.**

This is a **fundamental protocol design flaw** that makes the sync system unreliable for multi-device use. The plan's mitigation in the Risks table says:

> "Track max seq across all downloaded ops, not just from one device"

This mitigation **does not solve the problem** — it *is* the problem.

### What should happen
1. **Option A:** Change the server endpoint to accept a `device_id` parameter and query per-device, or accept a map of `{device_id: last_seq}`.
2. **Option B:** Add a `server_seq` (global auto-increment) column to the `operations` table on the backend and query by that instead.
3. **Option C:** Query by `created_at` (timestamp) rather than `seq`.

Without changing the server endpoint or the sync query strategy, the multi-device sync will silently drop operations. **This must be fixed before Phase 8 is considered complete.**

### Note on the existing `sync_state` table
The codebase already has a `sync_state` table in `migrations/0002_operations.sql`:

```sql
CREATE TABLE IF NOT EXISTS sync_state (
    user_id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    last_local_seq INTEGER NOT NULL DEFAULT 0,
    last_synced_seq INTEGER NOT NULL DEFAULT 0,
    last_sync_at TEXT
);
```

However, `operation_store.rs` does **not** read or write this table. Even if it did, the single `last_synced_seq` column would still be structurally insufficient because it only tracks one number, not one per remote device.

---

## 3. Plaintext Password in `config.json` — Security Debt

**Severity:** Note (elevated to Blocker for production, acceptable for Phase 8 dev)

### What the plan says
```json
{
  "email": "dev@todomrs.io",
  "password": "password123"
}
```

### Problems
- The file is written to `~/.config/todomrs/config.json` with **default filesystem permissions** (typically `644` on Unix, world-readable).
- No file permission hardening (e.g., `chmod 600`) is performed in `Config::save()`.
- The `password` field is stored in **plaintext JSON**.

### Better approach
- **Phase 8:** Acceptable for dev/iteration if documented as a risk. The plan does not document this as a risk.
- **Future:** Use the OS keyring (e.g., `keyring` crate) or at least store a refresh token instead of the raw password. Prompt for password on first run and exchange it for a long-lived token stored securely.
- **Immediate fix:** Add `chmod 600` to `Config::save()` so the file is at least not world-readable.

---

## 4. Blocking Sync on Startup — Network-Down UX Is Poor

**Severity:** Note

### What the plan says
> "Create SyncClient and login (before terminal enters raw mode — so errors are visible)" and "Call `app.sync()` for initial sync after `refresh_tasks()`"

### Problems
- If the network is down or Supabase is unreachable, `login()` and `sync()` will **block** until TCP timeout (typically 30–75 seconds).
- The terminal is in raw mode by the time `app.sync()` runs, so `eprintln!` output from the sync method may garble the TUI or be invisible.
- The user cannot interact with the app while the sync is pending.

### What the plan misses
- No timeout on the `reqwest` client (default is 30s, but no explicit override).
- No background/async task pattern. The sync is inline on the main async loop.

### Recommendation
- The sync should be **non-blocking** on startup: initialize the client, then kick off `sync()` in a `tokio::spawn` task, or use a short timeout (e.g., 5s) for the initial sync and retry in the background.
- The status bar can show "↻" while the background sync runs.

---

## 5. Skipping Own Ops + `last_synced_seq` Logic — Flawed

**Severity:** Blocker

### What the plan says
```rust
for op in &remote_ops {
    if op.device_id != self.device_id {  // Skip our own ops
        apply_remote_operation(op)
    }
    if op.seq > self.last_synced_seq {
        self.last_synced_seq = op.seq;
    }
}
```

### Why this is wrong
The plan **advances `last_synced_seq` even for its own ops**. Consider:

1. Device A creates 50 ops → `last_synced_seq` = 50 after sync.
2. Device A calls `get_operations(50)`.
3. The server returns ops with `seq > 50` from **all** devices.
4. Device B has only created ops with `seq = 1..10`.
5. **Device B's ops are never downloaded** because they are permanently below `last_synced_seq`.

This compounds the per-device seq problem from §2. The plan's own-device skip logic is correct (don't apply your own ops), but the seq advancement across all devices is wrong.

### Correct approach
If the server endpoint were fixed to support per-device queries, the client should track `last_synced_seq` **per remote device_id**, not as a single global number. For example:

```rust
HashMap<Uuid, i64> last_seq_by_device;
```

---

## 6. Error Handling — Silent Failures in Raw Mode

**Severity:** Note

### What the plan says
```rust
if let Err(e) = self.apply_remote_operation(op).await {
    eprintln!("Failed to apply remote op {:?}: {}", op.op_id, e);
}
```

### Problems
- `eprintln!` during terminal raw mode is **not visible** to the user. It writes to stderr, but the alternate screen buffer is active. The user sees nothing.
- `sync_status = SyncStatus::Offline(format!("..."))` updates the status bar, but if the error is a transient network blip, the user may miss the status change.
- There is **no persistent log file** or structured logging. Errors are lost when the app exits.

### Recommendations
- Use a real logging framework (e.g., `tracing` or `log` + `env_logger`) so errors go to a file or stderr that can be inspected after the app exits.
- Keep the most recent error in `status_message` so it is visible in the UI for more than one frame.
- Consider adding a small "last error" timestamp so the user knows if the offline status is stale.

---

## 7. Tag Operations Not Handled — Acceptable but Must Be Documented

**Severity:** Note

### What the plan says
```rust
_ => {} // Tag operations, reminders — skip for now
```

### Verification
- `operations.rs` defines `Entity::Tag`, `OperationPayload::TagCreate`, and `OperationPayload::TagUpdate`.
- The local schema has `tags` and `task_tags` tables.
- However, the TUI does **not** expose tag management in the UI (no create/edit/delete tag commands). Tags are only used via `task.tag_ids` in the task store.

### Why this is acceptable for Phase 8
- The app does not support tag editing natively, so remote tag operations would have no local UI representation.
- Task operations that include `tag_ids` in the payload will still create the task, but `set_task_tags` will fail with a **foreign key constraint violation** if the referenced tags don't exist locally.

### Risk
If a remote device creates a task with `tag_ids: [tag1, tag2]`, and those tags don't exist locally, `task_store.create()` will **crash** because `task_tags` has `FOREIGN KEY (tag_id) REFERENCES tags(id)` (`migrations/0001_init.sql`). The plan's idempotency check (`if self.task_store.get_by_id(op.entity_id).await?.is_some() { return Ok(()); }`) only helps if the task already exists locally. For a genuinely new task with unknown tags, the apply will fail.

### Recommendation
- **Document** that Phase 8 intentionally skips Tag and Reminder operations.
- **Add** a defensive check in `apply_remote_operation` for `TaskCreate`: if `tag_ids` references non-existent tags, either skip the tag assignment or create stub tags. Do not let the FK constraint crash the sync loop.

---

## Additional Findings

### A. `config_path()` visibility inconsistency
The plan defines `config_path()` as `fn` (private) but says "Make `config_path()` public on Config so the error message can show the path." The plan should be consistent: either make it `pub fn` in the code snippet or remove the requirement from the wiring instructions.

### B. `recurrence_store.get()` vs `get_by_id()`
The plan says:
> "Check if `recurrence_store` has a `get(id)` method. If not, add"

The actual `recurrence_store.rs` already has `get_by_id(id)`. The plan should use `get_by_id` instead of inventing a new `get()` method.

### C. `Deserialize` not derived on `App` but `Debug` is hand-rolled
Adding `sync_client: Option<SyncClient>` to `App` is fine because the struct already has `#[allow(dead_code)]`. However, the hand-rolled `Debug` impl in `app.rs` does not list `recurrence_rules`, `project_selected_index`, etc. Adding `sync_client` won't break the existing `Debug` impl, but it means the new fields won't appear in debug output. Not a blocker.

### D. `get_operations` server endpoint bug (not in plan scope but worth noting)
The server endpoint `get-operations` does not filter by `device_id`, and it uses `.gt('seq', ...)` globally. Even if the client fixes its `last_synced_seq` logic, the server endpoint is still broken for per-device sequence numbers. This requires a backend change.

---

## Summary

| # | Finding | Severity | Location |
|---|---------|----------|----------|
| 1 | `Option<SyncClient>` is correct | ✅ Correct | `plan.md` §2 |
| 2 | `last_synced_seq` in-memory is okay, but protocol design loses ops | 🔴 Blocker | `plan.md` §3, server `get-operations` |
| 3 | Plaintext password in config | 🟡 Note | `plan.md` §1, `config.rs` |
| 4 | Blocking startup sync | 🟡 Note | `plan.md` §5, `main.rs` |
| 5 | `last_synced_seq` advancement across devices is wrong | 🔴 Blocker | `plan.md` §4 |
| 6 | Error handling invisible in raw mode | 🟡 Note | `plan.md` §4 |
| 7 | Tag ops skipped — acceptable but FK risk exists | 🟡 Note | `plan.md` §4 |
| A | `config_path()` visibility inconsistency | 🟡 Note | `plan.md` §1, §5 |
| B | `recurrence_store.get()` already exists as `get_by_id()` | 🟡 Note | `plan.md` §8 |
| D | Server `get-operations` uses global seq over per-device seq | 🔴 Blocker | `supabase/functions/get-operations/index.ts` |

### Critical Action Items
1. **Fix the server endpoint** to support per-device sequence tracking (or switch to global `server_seq` / `created_at` query).
2. **Fix the client sync logic** to track `last_synced_seq` per remote `device_id`, not as a single global number.
3. **Add FK-safety** for remote task creates that reference unknown tags.
4. **Harden config file permissions** (at minimum `chmod 600`).
5. **Document** the Tag/Reminder skip in the plan's Risks table.
