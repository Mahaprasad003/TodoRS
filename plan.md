# Phase 8: TUI Sync Client Integration — Implementation Plan

## Overview

Integrate the Phase 7 `SyncClient` into the TUI so that every local mutation (task create/edit/complete/delete, project create/delete, recurrence changes) uploads to Supabase, and remote operations from other devices are downloaded and applied locally.

---

## Key Design Decisions

### 1. Config Approach: `~/.config/todomrs/config.json`

Use a JSON config file rather than `.env` (which is a dev-only artifact). On first run, create the file with placeholder values. The user fills in their Supabase URL, anon key, email, and password. This is portable, doesn't require a `dotenv` crate, and follows standard TUI app conventions.

```json
{
  "supabase_url": "https://sjtinieirxibsukjleta.supabase.co",
  "supabase_api_key": "YOUR_ANON_KEY",
  "email": "dev@todomrs.io",
  "password": "password123"
}
```

**New file:** `crates/todomrs-tui/src/config.rs`

### 2. App Struct Changes

Add three new fields to `App` — do **not** change the `App::new()` signature (keep it backward-compatible). Instead, add a `init_sync()` async method called from `main.rs` after construction.

```rust
pub enum SyncStatus {
    Disabled,       // No config / login failed
    Syncing,
    Synced,
    Offline(String), // Error message
}

// New fields on App:
pub sync_client: Option<SyncClient>,
pub sync_status: SyncStatus,
pub last_synced_seq: i64,
```

### 3. Sync Flow

```
sync() {
    sync_status = Syncing
    
    // 1. Upload local unsynced operations
    unsynced = op_store.get_unsynced(user_id)
    if !unsynced.is_empty():
        sync_client.upload_operations(unsynced.clone())
        mark_synced(op_ids)
    
    // 2. Download remote operations (from other devices)
    remote_ops = sync_client.get_operations(last_synced_seq)
    for op in remote_ops:
        if op.device_id != self.device_id:  // Skip our own ops
            apply_remote_operation(op)
        if op.seq > last_synced_seq:
            last_synced_seq = op.seq
    
    // 3. Refresh UI
    refresh_tasks()
    sync_status = Synced
}
```

### 4. Remote Operation Application

Skip ops from our own device (`op.device_id == self.device_id`). For remote ops:

| Operation | Local Action |
|-----------|-------------|
| Task Create | `task_store.create()` if not exists |
| Task Update | `task_store.get_by_id()` → apply fields → `task_store.update()` |
| Task Delete | `task_store.soft_delete()` |
| Project Create | `project_store.create()` if not exists |
| Project Update | `project_store.get_by_id()` → apply → `project_store.update()` |
| RecurrenceRule Create | `recurrence_store.create()` if not exists |
| RecurrenceRule Update | `recurrence_store.get()` → apply → `recurrence_store.update()` |
| RecurrenceRule Delete | `recurrence_store.delete()` |
| Delete (generic) | `task_store.soft_delete()` |

**Idempotency:** All applies check existence before creating/updating. "Already exists" is not an error.

### 5. SyncClient Enhancement

The current `SyncClient` doesn't expose the `access_token` getter or an `is_authenticated()` check. Add:

```rust
pub fn is_authenticated(&self) -> bool {
    self.access_token.is_some()
}
```

---

## Task Breakdown

### Task 1: Add `config.rs` module

**File:** `crates/todomrs-tui/src/config.rs` (new)

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub supabase_url: String,
    pub supabase_api_key: String,
    pub email: String,
    pub password: String,
}

impl Config {
    /// Load config from ~/.config/todomrs/config.json.
    /// Creates the file with placeholder values if it doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config {
                supabase_url: "https://YOUR_PROJECT.supabase.co".to_string(),
                supabase_api_key: "YOUR_ANON_KEY".to_string(),
                email: "".to_string(),
                password: "".to_string(),
            };
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".config")
            .join("todomrs")
            .join("config.json")
    }

    /// Check if config has real credentials (not placeholders).
    pub fn is_configured(&self) -> bool {
        !self.supabase_url.contains("YOUR_PROJECT")
            && !self.supabase_api_key.contains("YOUR_ANON_KEY")
            && !self.email.is_empty()
            && !self.password.is_empty()
    }
}
```

**File:** `crates/todomrs-tui/src/main.rs` — add `mod config;`

**Acceptance:** `cargo build` passes. Config file created at `~/.config/todomrs/config.json` on first run.

---

### Task 2: Add SyncClient `is_authenticated()` method

**File:** `crates/todomrs-sync/src/client.rs`

Add one method to `SyncClient`:

```rust
pub fn is_authenticated(&self) -> bool {
    self.access_token.is_some()
}
```

**Acceptance:** `cargo build` passes.

---

### Task 3: Add sync fields to App struct

**File:** `crates/todomrs-tui/src/app.rs`

Add `SyncStatus` enum and new fields:

```rust
use todomrs_sync::SyncClient;

#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    Disabled,
    Syncing,
    Synced,
    Offline(String),
}

// Add to App struct (after recurrence_rules field):
pub sync_client: Option<SyncClient>,
pub sync_status: SyncStatus,
pub last_synced_seq: i64,
```

Initialize in `App::new()`:

```rust
sync_client: None,
sync_status: SyncStatus::Disabled,
last_synced_seq: 0,
```

Add `init_sync_client()` method:

```rust
pub fn set_sync_client(&mut self, client: SyncClient) {
    self.sync_client = Some(client);
}
```

**Acceptance:** `cargo build` passes. No behavioral change yet.

---

### Task 4: Implement `sync()` and `apply_remote_operation()` methods

**File:** `crates/todomrs-tui/src/app.rs`

Add two methods to `impl App`:

```rust
/// Perform a full sync cycle: upload local ops, download remote ops, apply them.
pub async fn sync(&mut self) -> Result<()> {
    let client = match &self.sync_client {
        Some(c) if c.is_authenticated() => c,
        _ => {
            self.sync_status = SyncStatus::Disabled;
            return Ok(());
        }
    };

    self.sync_status = SyncStatus::Syncing;

    // 1. Upload local unsynced operations
    match self.op_store.get_unsynced(self.user_id).await {
        Ok(unsynced) if !unsynced.is_empty() => {
            if let Err(e) = client.upload_operations(unsynced.clone()).await {
                self.sync_status = SyncStatus::Offline(format!("Upload failed: {}", e));
                return Ok(());
            }
            let op_ids: Vec<Uuid> = unsynced.iter().map(|op| op.op_id).collect();
            self.op_store.mark_synced(&op_ids).await?;
        }
        Ok(_) => {} // Nothing to upload
        Err(e) => {
            self.sync_status = SyncStatus::Offline(format!("DB error: {}", e));
            return Ok(());
        }
    }

    // 2. Download remote operations
    let remote_ops = match client.get_operations(self.last_synced_seq).await {
        Ok(ops) => ops,
        Err(e) => {
            self.sync_status = SyncStatus::Offline(format!("Download failed: {}", e));
            return Ok(());
        }
    };

    // 3. Apply remote operations (skip our own)
    for op in &remote_ops {
        if op.device_id != self.device_id {
            if let Err(e) = self.apply_remote_operation(op).await {
                // Log but don't fail — skip problematic ops
                eprintln!("Failed to apply remote op {:?}: {}", op.op_id, e);
            }
        }
        if op.seq > self.last_synced_seq {
            self.last_synced_seq = op.seq;
        }
    }

    // 4. Refresh UI
    self.refresh_tasks().await?;

    if remote_ops.is_empty() {
        self.sync_status = SyncStatus::Synced;
    } else {
        let applied = remote_ops.iter().filter(|op| op.device_id != self.device_id).count();
        self.sync_status = SyncStatus::Synced;
        self.status_message = Some(format!("Synced ({} remote ops)", applied));
    }

    Ok(())
}

/// Apply a single remote operation to the local database.
async fn apply_remote_operation(&mut self, op: &Operation) -> Result<()> {
    use todomrs_sync::operations::{Entity, OperationPayload, OperationType};

    match (&op.entity, &op.op_type) {
        // ── Task Create ───────────────────────────────────────────
        (Entity::Task, OperationType::Create) => {
            if let OperationPayload::TaskCreate {
                title, description, status, project_id, tag_ids,
                priority, due_at, scheduled_at, recurrence_rule_id,
            } = &op.payload {
                // Skip if already exists (idempotent)
                if self.task_store.get_by_id(op.entity_id).await?.is_some() {
                    return Ok(());
                }
                let mut task = Task::new(self.user_id, title.clone());
                task.id = op.entity_id;
                task.description = description.clone();
                task.status = status.clone();
                task.project_id = *project_id;
                task.tag_ids = tag_ids.clone();
                task.priority = priority.clone();
                task.due_at = *due_at;
                task.scheduled_at = *scheduled_at;
                task.recurrence_rule_id = *recurrence_rule_id;
                task.created_at = op.created_at;
                task.updated_at = op.created_at;
                self.task_store.create(&task).await?;
            }
        }

        // ── Task Update ───────────────────────────────────────────
        (Entity::Task, OperationType::Update) => {
            if let Some(mut task) = self.task_store.get_by_id(op.entity_id).await? {
                if let OperationPayload::TaskUpdate {
                    title, description, status, project_id, tag_ids,
                    priority, due_at, scheduled_at, recurrence_rule_id,
                    completed_at,
                } = &op.payload {
                    if let Some(t) = title { task.title = t.clone(); }
                    if let Some(d) = description { task.description = Some(d.clone()); }
                    if let Some(s) = status { task.status = s.clone(); }
                    if let Some(p) = project_id { task.project_id = Some(*p); }
                    if let Some(t) = tag_ids { task.tag_ids = t.clone(); }
                    if let Some(p) = priority { task.priority = p.clone(); }
                    if let Some(d) = due_at { task.due_at = Some(*d); }
                    if let Some(d) = scheduled_at { task.scheduled_at = Some(*d); }
                    if let Some(r) = recurrence_rule_id { task.recurrence_rule_id = Some(*r); }
                    if let Some(c) = completed_at { task.completed_at = Some(*c); }
                    task.updated_at = op.created_at;
                    self.task_store.update(&task).await?;
                }
            }
            // If task doesn't exist locally, skip (may have been deleted)
        }

        // ── Task Delete ───────────────────────────────────────────
        (Entity::Task, OperationType::Delete) => {
            self.task_store.soft_delete(op.entity_id).await?;
        }

        // ── Project Create ────────────────────────────────────────
        (Entity::Project, OperationType::Create) => {
            if let OperationPayload::ProjectCreate { name, color, sort_order } = &op.payload {
                if self.project_store.get_by_id(op.entity_id).await?.is_some() {
                    return Ok(());
                }
                let now = chrono::Utc::now();
                let project = Project {
                    id: op.entity_id,
                    user_id: self.user_id,
                    name: name.clone(),
                    color: color.clone(),
                    sort_order: *sort_order,
                    created_at: op.created_at,
                    updated_at: op.created_at,
                    archived_at: None,
                };
                self.project_store.create(&project).await?;
            }
        }

        // ── Project Update ────────────────────────────────────────
        (Entity::Project, OperationType::Update) => {
            if let Some(mut project) = self.project_store.get_by_id(op.entity_id).await? {
                if let OperationPayload::ProjectUpdate { name, color, sort_order, archived_at } = &op.payload {
                    if let Some(n) = name { project.name = n.clone(); }
                    if let Some(c) = color { project.color = Some(c.clone()); }
                    if let Some(s) = sort_order { project.sort_order = *s; }
                    if let Some(a) = archived_at { project.archived_at = Some(*a); }
                    project.updated_at = op.created_at;
                    self.project_store.update(&project).await?;
                }
            }
        }

        // ── RecurrenceRule Create ─────────────────────────────────
        (Entity::RecurrenceRule, OperationType::Create) => {
            if let OperationPayload::RecurrenceRuleCreate {
                task_id, kind, interval, timezone,
                wait_for_completion, anchor_mode,
            } = &op.payload {
                if self.recurrence_store.get(op.entity_id).await?.is_some() {
                    return Ok(());
                }
                let rule = todomrs_core::domain::RecurrenceRule {
                    id: op.entity_id,
                    task_id: *task_id,
                    kind: deserialize_recurrence_kind(kind),
                    interval: *interval,
                    by_weekday: None,
                    by_monthday: None,
                    timezone: timezone.clone(),
                    wait_for_completion: *wait_for_completion,
                    anchor_mode: deserialize_anchor_mode(anchor_mode),
                    created_at: op.created_at,
                    updated_at: op.created_at,
                };
                self.recurrence_store.create(&rule).await?;
            }
        }

        // ── RecurrenceRule Update ─────────────────────────────────
        (Entity::RecurrenceRule, OperationType::Update) => {
            if let Some(mut rule) = self.recurrence_store.get(op.entity_id).await? {
                if let OperationPayload::RecurrenceRuleUpdate {
                    interval, wait_for_completion, anchor_mode,
                } = &op.payload {
                    if let Some(i) = interval { rule.interval = *i; }
                    if let Some(w) = wait_for_completion { rule.wait_for_completion = *w; }
                    if let Some(a) = anchor_mode { rule.anchor_mode = deserialize_anchor_mode(a); }
                    rule.updated_at = op.created_at;
                    self.recurrence_store.update(&rule).await?;
                }
            }
        }

        // ── RecurrenceRule Delete ─────────────────────────────────
        (Entity::RecurrenceRule, OperationType::Delete) => {
            self.recurrence_store.delete(op.entity_id).await?;
        }

        // ── Generic Delete (fallback) ─────────────────────────────
        (_, OperationType::Delete) => {
            // Best-effort: try soft-delete as task
            self.task_store.soft_delete(op.entity_id).await.ok();
        }

        _ => {} // Tag operations, reminders — skip for now
    }

    Ok(())
}

// ── Helper deserializers ─────────────────────────────────────────────

fn deserialize_recurrence_kind(s: &str) -> todomrs_core::domain::RecurrenceKind {
    match s.to_lowercase().as_str() {
        "daily" => todomrs_core::domain::RecurrenceKind::Daily,
        "weekly" => todomrs_core::domain::RecurrenceKind::Weekly,
        "monthly" => todomrs_core::domain::RecurrenceKind::Monthly,
        "yearly" => todomrs_core::domain::RecurrenceKind::Yearly,
        _ => todomrs_core::domain::RecurrenceKind::Daily,
    }
}

fn deserialize_anchor_mode(s: &str) -> todomrs_core::domain::AnchorMode {
    match s.to_lowercase().as_str() {
        "completion" => todomrs_core::domain::AnchorMode::Completion,
        _ => todomrs_core::domain::AnchorMode::Schedule,
    }
}
```

**Acceptance:** `cargo build` passes. Methods exist but are not yet called.

---

### Task 5: Wire sync into `main.rs`

**File:** `crates/todomrs-tui/src/main.rs`

Changes:
1. Add `mod config;`
2. Load config before terminal setup
3. Create SyncClient and login (before terminal enters raw mode — so errors are visible)
4. Pass sync_client to App
5. Call `app.sync()` for initial sync after `refresh_tasks()`
6. Print config path if sync is disabled

```rust
mod app;
mod ui;
mod config;

// In run_async(), BEFORE terminal setup:
let config = config::Config::load()?;

// Create sync client
let sync_client = if config.is_configured() {
    let mut client = todomrs_sync::SyncClient::new(
        config.supabase_url.clone(),
        config.supabase_api_key.clone(),
    );
    match client.login(&config.email, &config.password).await {
        Ok(_) => Some(client),
        Err(e) => {
            eprintln!("Sync login failed: {}", e);
            None
        }
    }
} else {
    eprintln!("Sync not configured. Edit: {}", config::Config::config_path().display());
    None
};

// After App::new():
if let Some(client) = sync_client {
    app.set_sync_client(client);
}

// After app.refresh_tasks():
if app.sync_client.is_some() {
    app.sync().await?;
}
```

**Important:** Make `config_path()` public on Config so the error message can show the path.

**Acceptance:** `cargo build` passes. On first run, config file is created. With valid credentials, initial sync runs.

---

### Task 6: Add 'S' key binding for manual sync

**File:** `crates/todomrs-tui/src/app.rs`

In `handle_event()`, in the `InputMode::Normal` match block, add:

```rust
KeyCode::Char('S') if key.modifiers.is_empty() => {
    self.sync().await?;
}
```

Add it after the existing `KeyCode::Char('C')` handler.

**Acceptance:** Pressing 'S' in Normal mode triggers sync.

---

### Task 7: Update UI — sync status indicator

**File:** `crates/todomrs-tui/src/ui.rs`

#### 7a. Status bar — add sync indicator

After the view name span and project filter indicator, add a sync status span:

```rust
// In draw_status_bar(), after the [P] indicator span:
if let Some(ref _client) = app.sync_client {
    // Show sync status
    let (sync_text, sync_color) = match &app.sync_status {
        SyncStatus::Disabled => ("○", Color::DarkGray),
        SyncStatus::Syncing => ("↻", Color::Yellow),
        SyncStatus::Synced => ("✓", Color::Green),
        SyncStatus::Offline(_) => ("✗", Color::Red),
    };
    status_spans.push(Span::styled(
        format!(" {} ", sync_text),
        Style::default().fg(sync_color),
    ));
}
```

Also add 'S' shortcut to the status bar (after the Search shortcut):

```rust
Span::styled("S", Style::default().fg(Color::Yellow)),
Span::raw(" Sync "),
```

#### 7b. Help text — add sync section

Add after "Search & Help:" section:

```rust
Line::from(""),
Line::from("Sync:"),
Line::from("  S      — Sync now"),
Line::from("  ✓ = synced  ↻ = syncing  ✗ = offline  ○ = disabled"),
```

**Acceptance:** Status bar shows sync indicator. Help text includes sync shortcuts.

---

### Task 8: Handle `recurrence_store.get()` — check if it exists

**File:** `crates/todomrs-store/src/recurrence_store.rs`

Check if `recurrence_store` has a `get(id)` method. If not, add:

```rust
pub async fn get(&self, id: Uuid) -> Result<Option<RecurrenceRule>> {
    // SELECT * FROM recurrence_rules WHERE id = ?
}
```

**Acceptance:** `cargo build` passes.

---

### Task 9: Add `last_synced_seq` persistence

The `last_synced_seq` is currently in-memory only. On restart, it resets to 0, which means we'd re-download all operations. This is fine for now (idempotent apply skips duplicates), but for efficiency, persist it.

**Approach for Phase 8:** Keep it in-memory. The operation application is idempotent, so re-downloading is safe. Optimize in a later phase by storing `last_synced_seq` in the `sync_state` SQLite table.

**No changes needed for this task — document as future optimization.**

---

## Files Modified (Summary)

| File | Changes |
|------|---------|
| `crates/todomrs-tui/src/config.rs` | **NEW** — Config struct, load/save from `~/.config/todomrs/config.json` |
| `crates/todomrs-tui/src/main.rs` | Add `mod config`, load config, init SyncClient, login, initial sync |
| `crates/todomrs-tui/src/app.rs` | Add `SyncStatus` enum, `sync_client`/`sync_status`/`last_synced_seq` fields, `set_sync_client()`, `sync()`, `apply_remote_operation()`, 'S' key binding, helper deserializers |
| `crates/todomrs-tui/src/ui.rs` | Add sync indicator to status bar, add 'S Sync' shortcut, add sync section to help |
| `crates/todomrs-sync/src/client.rs` | Add `is_authenticated()` method |
| `crates/todomrs-store/src/recurrence_store.rs` | Add `get(id)` method if missing |

## Files NOT Modified

- `crates/todomrs-sync/src/operations.rs` — no changes needed
- `crates/todomrs-sync/src/snapshot.rs` — no changes needed
- `crates/todomrs-core/*` — no changes needed
- `Cargo.toml` (workspace) — no new deps needed
- `crates/todomrs-tui/Cargo.toml` — no new deps needed (serde/serde_json already there)

---

## Testing Plan

### Unit Tests
- Config: test `load()` creates file, `save()` writes, `is_configured()` checks placeholders
- SyncClient: `is_authenticated()` returns false before login, true after

### Integration Tests (manual)
1. **First run without config:** App starts, shows "○" sync indicator, prints config path
2. **Configure and restart:** App starts, logs in, runs initial sync, shows "✓"
3. **Create task on device A, sync, create task on device B, sync:** Both tasks appear on both devices
4. **Network failure:** Sync shows "✗" with error, app doesn't crash
5. **Idempotent sync:** Sync twice in a row — no duplicate tasks

### Verification Commands
```bash
# Build
cargo build --bin todomrs

# Run
cargo run --bin todomrs

# Check config was created
cat ~/.config/todomrs/config.json
```

---

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| SyncClient `Operation` deserialization from server JSON may differ from local format | Server stores same JSON structure — verified in Phase 7 e2e test |
| Remote ops may reference entities that don't exist locally (e.g., Task Update for unknown task) | Idempotent apply: skip if entity not found |
| Sequence numbers are per-device — `last_synced_seq` tracking needs care | Track max seq across all downloaded ops, not just from one device |
| Config file permissions | `~/.config/todomrs/` is user-owned, not world-readable by default |
| SyncClient is not Clone — can't easily share between threads | Only used from main async task — no concurrency issue |

---

## Implementation Order

1. Task 2 (SyncClient `is_authenticated`) — trivial, unblocks everything
2. Task 8 (recurrence_store.get) — check if exists, add if needed
3. Task 1 (config.rs) — standalone, no deps
4. Task 3 (App sync fields) — adds fields, no behavior change
5. Task 4 (sync + apply methods) — core logic
6. Task 5 (main.rs wiring) — connects everything
7. Task 6 ('S' key binding) — one line
8. Task 7 (UI changes) — status bar + help text
