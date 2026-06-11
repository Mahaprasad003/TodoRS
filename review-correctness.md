## Review — Phase 8 Sync Integration Plan vs. Current Codebase

### Methodology
Read every file referenced in the plan (`app.rs`, `main.rs`, `ui.rs`, `client.rs`, `operations.rs`, `lib.rs`, `operation_store.rs`, `recurrence_store.rs`, `task_store.rs`, `project_store.rs`, `domain.rs`, `Cargo.toml` workspace) and compared the plan's claimed signatures/fields against the actual code.

---

## Correct

### 1. Core types and signatures exist
- `App::new()` signature **is unchanged** in the plan — the plan adds fields with default values inline. `cargo check` passes on the current tree.
- `OperationStore::{get_unsynced, mark_synced, get_next_seq, append}` all exist (`crates/todomrs-store/src/operation_store.rs:56`, `71`, `86`, `15`).
- `TaskStore::{create, get_by_id, update, soft_delete}` all exist (`crates/todomrs-store/src/task_store.rs:20`, `84`, `108`, `160`).
- `ProjectStore::{create, get_by_id, update, soft_delete, find_by_name}` all exist (`crates/todomrs-store/src/project_store.rs:17`, `37`, `52`, `98`, `82`).
- `SyncClient::{new, login, upload_operations, get_operations}` all exist (`crates/todomrs-sync/src/client.rs:15`, `31`, `53`, `79`).
- `todomrs_sync::operations::{Operation, Entity, OperationType, OperationPayload}` all exist and match the variants the plan uses (`crates/todomrs-sync/src/operations.rs`).
- `Task`, `Project`, `RecurrenceRule`, `TaskStatus`, `Priority`, `RecurrenceKind`, `AnchorMode` all have the fields the plan expects (`crates/todomrs-core/src/domain.rs`).
- `Task::new(user_id, title)` exists and returns a struct with `id: Uuid::new_v4()`. The plan overwrites `task.id = op.entity_id` immediately after — this is correct and preserves the remote ID.
- `Config` does not need new crates: `serde` and `serde_json` are already in `todomrs-tui/Cargo.toml`.
- `Operation::create_task`, `Operation::complete_task`, `Operation::create_recurrence_rule` all exist and are used by the existing code exactly as the plan assumes.

### 2. Sync flow will NOT double-upload or conflict
- Every local mutation already creates an `Operation` and appends it via `op_store.append(&op)`.
- `OperationStore::get_unsynced` filters on `synced_at IS NULL` (`operation_store.rs:56`).
- `OperationStore::mark_synced` writes `synced_at = now` (`operation_store.rs:71`).
- The plan's `sync()` uploads `get_unsynced`, then `mark_synced`, then downloads. This is the correct order; no local op will be uploaded twice because `synced_at` is set. **No regression**.

### 3. Idempotency of remote operation application
- `TaskCreate` checks `get_by_id().is_some()` and skips if already present — idempotent.
- `ProjectCreate` checks `get_by_id().is_some()` and skips — idempotent.
- `Task Delete` calls `soft_delete()` which is an `UPDATE` — idempotent even if already deleted.
- `Task Update` blindly applies field mutations. Re-applying the same payload yields the same final state (except `updated_at` is overwritten with `op.created_at` each time, which is acceptable).
- `RecurrenceRule Delete` calls `recurrence_store.delete()` which executes `DELETE FROM recurrence_rules WHERE id = ?`. On SQLite this returns `Ok(())` even when 0 rows match — idempotent.

### 4. 'S' key binding has no conflict
- Existing Normal-mode bindings: `q`, `?`, `a`, `e`, `/`, `j`, `k`, `x`, `d`, `c`, `C`, `1`–`6`, `Enter`, `Esc`. `S` is unbound. The plan's `KeyCode::Char('S') if key.modifiers.is_empty()` is safe (no conflict with `s` since `s` is not bound).

### 5. SyncStatus::Disabled vs Offline distinction is coherent
- `Disabled` is set when `sync_client` is `None` or `!is_authenticated()` — i.e., no credentials or login failed.
- `Offline(String)` is set when a network or DB error occurs during an active sync attempt.
- This distinction is sensible for the status bar: the user knows whether sync is *off* (not configured) or *broken* (temporarily unreachable).

### 6. `last_synced_seq` in-memory is acceptable
- The plan explicitly documents this as a known limitation for Phase 8 and notes that operation application is idempotent, so re-downloading on restart is safe. This is correct.

### 7. `config.rs` approach is sound (with minor notes below)
- `load()` creates a placeholder file on first run. `is_configured()` checks non-empty email/password and non-placeholder URL/key. This won't leak credentials on first run because the placeholder file contains no real secrets.

---

## Fixed / Blocker

### B1. `recurrence_store.get()` does NOT exist — plan uses wrong method name
**Location:** Plan Task 4 (`app.rs` sync methods) and Task 8 (`recurrence_store.rs`).
**Evidence:** `crates/todomrs-store/src/recurrence_store.rs` has `get_by_id` (line 37) and `find_by_task_id` (line 73), but **no `get`** method. `grep` confirms zero matches for `pub async fn get\b`.
**Impact:** The plan's `apply_remote_operation` contains three calls to `self.recurrence_store.get(op.entity_id)` (in RecurrenceRule Create, Update, and Delete branches). These will fail to compile.
**Fix:** Change all `recurrence_store.get(...)` to `recurrence_store.get_by_id(...)` in the plan (and in any implementation). Task 8 in the plan should be updated to say "add `get` if missing" — but since `get_by_id` already exists, Task 8 is actually unnecessary.

### B2. `Project Delete` remote operation is misapplied by the fallback delete handler
**Location:** Plan Task 4, `apply_remote_operation` generic delete fallback.
**Evidence:** The plan's fallback arm:
```rust
(_, OperationType::Delete) => {
    self.task_store.soft_delete(op.entity_id).await.ok();
}
```
**Impact:** If a remote device sends a `Project Delete` operation, the code attempts to `soft_delete` a **task** with the project's UUID. This will silently fail or corrupt data (a task row with that UUID may exist by coincidence, or the update will do nothing). There is no `Project Delete` case in the match.
**Fix:** Add an explicit `Entity::Project, OperationType::Delete` arm that calls `project_store.soft_delete(op.entity_id)`.

### B3. `TaskCreate` with non-existent `tag_ids` will crash due to FK constraint
**Location:** Plan Task 4, `TaskCreate` branch.
**Evidence:** `TaskStore::create` calls `set_task_tags` which inserts into `task_tags(task_id, tag_id)` (`task_store.rs:138`). If the remote task references tag IDs that do not exist in the local `tags` table, the SQLite insert will violate the FK constraint and the transaction will abort.
**Impact:** The entire `sync()` will fail because `apply_remote_operation` returns `Err`, but the plan's `sync()` only logs and continues: `eprintln!("Failed to apply remote op {:?}: {}", ...)` and moves on. However, the `task_store.create` call itself will return `Err` and the task won't be created. The bigger issue is that **sync is not resilient to missing tags** — the task silently doesn't appear.
**Fix:** Either skip tags that don't exist locally, or create them on-the-fly. The plan currently does neither.

### B4. `TaskCreate` with non-existent `project_id` or `recurrence_rule_id` will crash
**Location:** Plan Task 4, `TaskCreate` branch.
**Evidence:** `TaskStore::create` inserts `project_id` and `recurrence_rule_id` into the `tasks` table. If the remote operation references a project or recurrence rule that was created on another device but not yet synced locally, the row insert violates FK constraints.
**Impact:** Same as B3 — the task creation fails, and the user never sees the task.
**Fix:** The plan should either (a) create missing referenced projects/rules first, or (b) defer the task creation until dependencies arrive. For Phase 8, a simpler fix is to check `project_id` and `recurrence_rule_id` before creating: if the referenced row doesn't exist, set the field to `None` and log a warning.

### B5. `RecurrenceRuleCreate` references `task_id` that may not exist locally
**Location:** Plan Task 4, `RecurrenceRuleCreate` branch.
**Evidence:** `RecurrenceRule` has a FK to `tasks.id`. `RecurrenceRuleCreate` payload contains `task_id`. `recurrence_store.create` inserts this `task_id`. If the task hasn't been synced yet, the insert fails.
**Impact:** Same FK crash pattern as B3/B4.
**Fix:** Skip the rule if the referenced task doesn't exist locally, or create the rule with a placeholder task. Given the current architecture, skipping with a log is the safest Phase 8 approach.

---

## Note

### N1. Pre-existing gaps in local operation recording (not regressions, but the plan should acknowledge them)
- `delete_project` (`app.rs:686`) does **not** record a `Project Delete` operation. Local project deletions will never sync to other devices. The plan adds remote application logic for Project Delete, but local deletion still won't generate the op.
- Direct project creation from the Projects view (`handle_event` → `Editing` → `Enter` when `current_view == View::Projects`) does **not** record a `ProjectCreate` operation. Only project creation triggered inside `update_task_from_input` records an op.
- **Recommendation:** Add these two op recordings to the existing handlers before or alongside the sync integration, otherwise the sync will appear "one-way" for projects.

### N2. `TaskUpdate` payload can't distinguish "no change" from "set to None"
- The `OperationPayload::TaskUpdate` uses `Option<T>` for every field (`title: Option<String>`, `description: Option<String>`, etc.). This means a remote update cannot express "clear the description" versus "don't touch the description". If the field is `None`, the receiver skips it. This is a protocol-level limitation, not a plan bug, but it means metadata like `description` can never be explicitly cleared via sync.

### N3. Config file is not created with restrictive permissions
- `config.save()` uses `std::fs::write`. The resulting `~/.config/todomrs/config.json` may be world-readable depending on the user's umask. It contains a plaintext password. The plan should add `#[cfg(unix)]` permission tightening (`chmod 600`) after writing the file.

### N4. `Config::load()` will crash on malformed JSON
- If the user edits `config.json` and introduces a syntax error, `serde_json::from_str` will return `Err`. `main.rs` calls `config::Config::load()?` before the terminal is set up, so the user sees a raw Rust error message and the app exits. A more graceful approach is to print a friendly error like `"Config file is corrupted: {path}"` and continue with sync disabled.

### N5. `SyncClient::access_token` is private; `is_authenticated()` is genuinely missing
- The plan's Task 2 correctly identifies that `SyncClient` lacks `is_authenticated()`. Adding the one-liner `self.access_token.is_some()` is correct and unblocks the `sync()` guard.

### N6. `get_operations` may return ops our own device already created locally
- The plan correctly filters `if op.device_id != self.device_id` before applying. This is correct because our own unsynced ops were already uploaded in step 1, and we don't want to re-apply them locally.

### N7. `ui.rs` needs to import `SyncStatus`
- The current `ui.rs` imports `use crate::app::{App, InputMode, View};`. The plan's status-bar changes reference `app.sync_status` which is of type `SyncStatus`. The plan doesn't show updating the import, but it will be required: `use crate::app::SyncStatus;`. This is a trivial fix but should be noted in the plan.

### N8. `last_synced_seq` is updated for every downloaded op, including our own
- The plan does `if op.seq > self.last_synced_seq { self.last_synced_seq = op.seq; }` unconditionally. This is correct: we want to advance the watermark even for our own ops so we don't re-request them from the server.

### N9. `TaskCreate` application copies `op.created_at` into both `created_at` and `updated_at`
- This is correct for idempotency: it preserves the original remote timestamp rather than using the local `Utc::now()`.

---

## Summary

The plan is **architecturally sound** and the sync flow is correctly designed (no double-upload, proper idempotency, coherent status states). However, there are **five concrete blockers** that must be resolved before or during implementation:

1. `recurrence_store.get()` → `get_by_id()` (compilation error)
2. Add `Entity::Project, OperationType::Delete` arm (data corruption risk)
3. Guard `TaskCreate` against missing `tag_ids` FKs (silent task loss)
4. Guard `TaskCreate` against missing `project_id`/`recurrence_rule_id` FKs (silent task loss)
5. Guard `RecurrenceRuleCreate` against missing `task_id` FK (silent rule loss)

Additionally, the plan should note the pre-existing gaps in local operation recording for project create/delete, so the user doesn't expect two-way sync for projects immediately.
