# Code Context

## Project Overview

TodoRS is a personal task manager with a **Rust/ratatui TUI** (primary desktop interface) and a planned **SvelteKit PWA** (mobile client). It uses **operation-based sync** with a **Supabase backend** to keep multiple devices in sync. The project follows a phased implementation roadmap (Phases 1–10, with Phases 1–8 complete).

### Current State
- **Phase 8 complete** — TUI sync client integration with Supabase
- **Phase 9 not started** — PWA mobile client (SvelteKit)
- **Phase 10 planned** — Reminders, notifications, polish

### Key Git History (latest on main)
```
4f00a02 fix: debounce timer anchored to mutation time, not sync time
13908e1 fix: refresh UI immediately after mutation, defer sync to background
e39462c refactor: debounced auto-sync 10s after last mutation
1b7b5c4 feat: auto-sync on mutation, 30s periodic, and on exit
1bf6ae6 fix: always show sync status message (uploaded X, applied Y)
266ec1c fix: visible sync feedback in status bar with descriptive text
cda2ac2 feat: Phase 8 — TUI sync client integration
bc9598f feat: Phase 7 — Supabase backend setup with edge functions and Rust sync client
4d1360a feat: recurrence rule support and TUI recurrence integration
a33670f Phase 6.5: TUI Polish
...
70117c5 feat: initialize Cargo workspace with 4 crates
ea711ac first commit
```

Only `main` branch exists. Single remote (`origin/main`).

---

## Files Retrieved

### 1. Configuration & Build
- `/home/mp/Projects/TodoRS/Cargo.toml` (lines 1-13) — Workspace root with 4 crates, shared deps
- `/home/mp/Projects/TodoRS/.gitignore` (lines 1-9) — Ignores target, *.db, .env
- `/home/mp/Projects/TodoRS/.env` (lines 1-2) — Supabase URL + anon key (live project)
- `/home/mp/Projects/TodoRS/.todomrs_user_id` — Persistent local user UUID
- `/home/mp/Projects/TodoRS/.todomrs_device_id` — Persistent local device UUID

### 2. Core Domain (`crates/todomrs-core`)
- `/home/mp/Projects/TodoRS/crates/todomrs-core/Cargo.toml` — serde, chrono, uuid, thiserror
- `/home/mp/Projects/TodoRS/crates/todomrs-core/src/lib.rs` (lines 1-7) — Exposes domain, parser, recurrence
- `/home/mp/Projects/TodoRS/crates/todomrs-core/src/domain.rs` (lines 1-247) — Core types: Task, Project, Tag, Reminder, RecurrenceRule, enums
- `/home/mp/Projects/TodoRS/crates/todomrs-core/src/parser.rs` (lines 1-523) — NaturalLanguageParser, ParsedTask, date/time resolution
- `/home/mp/Projects/TodoRS/crates/todomrs-core/src/recurrence.rs` (lines 1-135) — RecurrenceEngine, next_occurrence computation

### 3. Store Layer (`crates/todomrs-store`)
- `/home/mp/Projects/TodoRS/crates/todomrs-store/Cargo.toml` — sqlx (sqlite), tokio, libsqlite3-sys
- `/home/mp/Projects/TodoRS/crates/todomrs-store/src/lib.rs` (lines 1-8) — Exports all stores
- `/home/mp/Projects/TodoRS/crates/todomrs-store/src/db.rs` (lines 1-30) — Database wrapper (SqlitePool, PRAGMAs)
- `/home/mp/Projects/TodoRS/crates/todomrs-store/src/task_store.rs` (lines 1-193) — Task CRUD, tag junction, enum serde helpers
- `/home/mp/Projects/TodoRS/crates/todomrs-store/src/project_store.rs` (lines 1-128) — Project CRUD, find_by_name, soft delete
- `/home/mp/Projects/TodoRS/crates/todomrs-store/src/tag_store.rs` (lines 1-80) — Tag CRUD
- `/home/mp/Projects/TodoRS/crates/todomrs-store/src/operation_store.rs` (lines 1-149) — Operation log CRUD, snapshots, seq tracking
- `/home/mp/Projects/TodoRS/crates/todomrs-store/src/recurrence_store.rs` (lines 1-220) — RecurrenceRule CRUD, tests

### 4. Sync Layer (`crates/todomrs-sync`)
- `/home/mp/Projects/TodoRS/crates/todomrs-sync/Cargo.toml` — reqwest for HTTP, serde
- `/home/mp/Projects/TodoRS/crates/todomrs-sync/src/lib.rs` (lines 1-4) — Exports operations, snapshot, client
- `/home/mp/Projects/TodoRS/crates/todomrs-sync/src/client.rs` (lines 1-140) — SyncClient: login, upload_operations, get_operations
- `/home/mp/Projects/TodoRS/crates/todomrs-sync/src/operations.rs` (lines 1-245) — Operation, Entity, OperationType, OperationPayload enums + constructors
- `/home/mp/Projects/TodoRS/crates/todomrs-sync/src/snapshot.rs` (lines 1-33) — Snapshot struct for state compaction

### 5. TUI (`crates/todomrs-tui`)
- `/home/mp/Projects/TodoRS/crates/todomrs-tui/Cargo.toml` — ratatui 0.26, crossterm 0.27, clap, tokio
- `/home/mp/Projects/TodoRS/crates/todomrs-tui/src/main.rs` (lines 1-112) — Entry point: config load, terminal setup, event loop, auto-sync
- `/home/mp/Projects/TodoRS/crates/todomrs-tui/src/app.rs` (lines 1-1537) — App struct, views, event handling, sync, remote ops, project mgmt
- `/home/mp/Projects/TodoRS/crates/todomrs-tui/src/ui.rs` (lines 1-343) — Terminal UI: sidebar, task list, input field, status bar, help overlay
- `/home/mp/Projects/TodoRS/crates/todomrs-tui/src/config.rs` (lines 1-62) — Config struct, load/save from ~/.config/todomrs/config.json

### 6. Migrations
- `/home/mp/Projects/TodoRS/migrations/0001_init.sql` — Core schema: users, projects, tags, tasks, task_tags, recurrence_rules, reminders
- `/home/mp/Projects/TodoRS/migrations/0002_operations.sql` — Operations and sync_state tables
- `/home/mp/Projects/TodoRS/migrations/0003_snapshots.sql` — Snapshots table
- `/home/mp/Projects/TodoRS/migrations/0004_recurrence_enhancements.sql` — ALTER TABLE for wait_for_completion + anchor_mode

### 7. Supabase Backend
- `/home/mp/Projects/TodoRS/supabase/config.toml` — Full Supabase local dev config (project_id = "TodoRS")
- `/home/mp/Projects/TodoRS/supabase/functions/get-operations/index.ts` — Edge function to fetch operations since timestamp
- `/home/mp/Projects/TodoRS/supabase/functions/upload-operations/index.ts` — Edge function to insert operations
- `/home/mp/Projects/TodoRS/backend/migrations/001_init.sql` — Postgres schema: operations, sync_state, snapshots, devices
- `/home/mp/Projects/TodoRS/backend/migrations/002_rls_policies.sql` — Row Level Security policies for all tables

### 8. Plans & Design
- `/home/mp/Projects/TodoRS/plan.md` — Phase 8 implementation plan (reference for sync integration)
- `/home/mp/Projects/TodoRS/Northstar.md` — Full product vision document
- `/home/mp/Projects/TodoRS/DESIGN-vercel.md` — Vercel-inspired design system reference for future PWA
- `/home/mp/Projects/TodoRS/RECURRENCE.md` — Recurrence feature documentation
- `/home/mp/Projects/TodoRS/plans/phase_09.md` — Phase 9 plan: PWA mobile client (SvelteKit)
- `/home/mp/Projects/TodoRS/plans/phase_10.md` — Phase 10 plan: Reminders, notifications, polish

---

## Key Code

### Core Domain Types (`crates/todomrs-core/src/domain.rs`)

```rust
pub struct Task {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,      // Pending | Completed
    pub project_id: Option<Uuid>,
    pub tag_ids: Vec<Uuid>,
    pub priority: Priority,      // None | Low | Medium | High | Urgent
    pub due_at: Option<DateTime<Utc>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub recurrence_rule_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

pub struct RecurrenceRule {
    pub id: Uuid,
    pub task_id: Uuid,
    pub kind: RecurrenceKind,    // Daily | Weekly | Monthly | Yearly
    pub interval: i32,
    pub by_weekday: Option<Vec<i32>>,
    pub by_monthday: Option<Vec<i32>>,
    pub timezone: String,
    pub wait_for_completion: bool,
    pub anchor_mode: AnchorMode, // Schedule | Completion
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Sync Operation Types (`crates/todomrs-sync/src/operations.rs`)

```rust
pub struct Operation {
    pub op_id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub seq: i64,
    pub entity: Entity,          // Task | Project | Tag | Reminder | RecurrenceRule
    pub entity_id: Uuid,
    pub op_type: OperationType,  // Create | Update | Delete
    pub payload: OperationPayload, // TaskCreate | TaskUpdate | ProjectCreate | ...
    pub created_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
}
```

### Natural Language Parser (`crates/todomrs-core/src/parser.rs`)

Parses input like `"Submit assignment +vit @writing due:friday p2 every week"` into:
- Title, project, tags, priority, due_date, due_time, recurrence, anchor mode
- `create_task_from_input()` returns `(Task, Option<RecurrenceRule>)`

### App State Machine (`crates/todomrs-tui/src/app.rs`)

Views: `Inbox | Today | Upcoming | Projects | Completed | Recurring`

Input modes: `Normal | Editing | EditingTask(Uuid) | Searching`

Sync status: `Disabled | Syncing | Synced | Offline(String)`

Auto-sync: debounced (10s after last mutation) + periodic (30s)

### UI Layout (`crates/todomrs-tui/src/ui.rs`)

- **Left sidebar**: View list (6 items) + Projects list with counts
- **Main content**: Filtered task list with priority/recurrence indicators
- **Bottom**: Input field + status bar with sync indicator and shortcuts
- **Help overlay**: Modal with keyboard shortcuts

### Supabase Edge Functions

- `get-operations`: Auth-gated, filters by `user_id` and `created_at > since`
- `upload-operations`: Auth-gated, inserts batch of operations

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        TUI (Rust/ratatui)                        │
│                                                                  │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────────────────┐   │
│  │ app.rs   │  │ ui.rs        │  │ main.rs                  │   │
│  │ - views  │  │ - sidebar    │  │ - terminal setup         │   │
│  │ - events │  │ - task list  │  │ - config load            │   │
│  │ - sync   │  │ - input bar  │  │ - sync init              │   │
│  │ - CRUD   │  │ - status bar │  │ - event loop             │   │
│  └────┬─────┘  └──────────────┘  └──────────────────────────┘   │
│       │                                                         │
│  ┌────▼────────────────────────────────────────────────────┐    │
│  │                   Store Layer (sqlx/SQLite)              │    │
│  │  ┌──────────┐ ┌───────────┐ ┌──────────┐ ┌──────────┐  │    │
│  │  │ TaskStore│ │ProjectStor│ │ TagStore │ │OpStore   │  │    │
│  │  │          │ │           │ │          │ │+Recur    │  │    │
│  │  └──────────┘ └───────────┘ └──────────┘ └──────────┘  │    │
│  └─────────────────────────────────────────────────────────┘    │
│       │                                                         │
│  ┌────▼────────────────────────────────────────────────────┐    │
│  │                Sync Client (reqwest)                     │    │
│  │  ┌─────────────────────────────────────────────────┐    │    │
│  │  │ SyncClient                                      │    │    │
│  │  │ - login(email, password)                        │    │    │
│  │  │ - upload_operations(ops)                        │    │    │
│  │  │ - get_operations(since_time)                    │    │    │
│  │  └──────────────────────┬──────────────────────────┘    │    │
│  └─────────────────────────┼────────────────────────────────┘    │
└────────────────────────────┼────────────────────────────────────┘
                             │ HTTPS
┌────────────────────────────▼────────────────────────────────────┐
│              Supabase Backend (Postgres + Edge Functions)        │
│                                                                  │
│  ┌──────────────┐  ┌─────────────────┐  ┌──────────────────┐   │
│  │ Auth         │  │ get-operations   │  │ upload-operations│   │
│  │ (Supabase    │  │ (Deno edge fn)   │  │ (Deno edge fn)   │   │
│  │  built-in)   │  │ SELECT * FROM    │  │ INSERT INTO      │   │
│  │              │  │ operations WHERE │  │ operations ...   │   │
│  │              │  │ user_id = ...    │  │                  │   │
│  └──────────────┘  └─────────────────┘  └──────────────────┘   │
│                                                                  │
│  Tables: operations, sync_state, snapshots, devices             │
│  RLS: All tables protected by auth.uid() policies               │
└──────────────────────────────────────────────────────────────────┘

Planned (Phase 9+):
┌──────────────────────────────────────────────────────────────────┐
│  PWA (SvelteKit + IndexedDB)                                     │
│  - Same sync protocol                                            │
│  - Offline via IndexedDB                                         │
│  - Service worker                                                │
└──────────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Mutation path**: User action → App.handle_event() → Store CRUD → Operation appended → Refresh UI → Debounced sync (10s)
2. **Sync path**: OpStore.get_unsynced() → SyncClient.upload_operations() → mark_synced() → SyncClient.get_operations(since) → apply_remote_operation() → Refresh UI
3. **Recurrence path**: Complete task with recurrence_rule_id → check anchor mode → RecurrenceEngine.next_occurrence() → spawn new task instance
4. **Startup**: Config.load() → SyncClient.login() → refresh_tasks() → initial sync() → event loop

### Database Schema (SQLite local)

- `users` (id, email, created_at, updated_at)
- `tasks` (id, user_id, title, description, status, project_id, priority, due_at, scheduled_at, recurrence_rule_id, created_at, updated_at, completed_at, deleted_at)
- `projects` (id, user_id, name, color, sort_order, created_at, updated_at, archived_at)
- `tags` (id, user_id, name, color, created_at, updated_at)
- `task_tags` (task_id, tag_id) — junction table
- `recurrence_rules` (id, task_id, kind, interval, by_weekday, by_monthday, timezone, wait_for_completion, anchor_mode, created_at, updated_at)
- `reminders` (id, task_id, remind_at, status, created_at, updated_at)
- `operations` (op_id, user_id, device_id, seq, entity, entity_id, op_type, payload, created_at, synced_at)
- `sync_state` (user_id, device_id, last_local_seq, last_synced_seq, last_sync_at)
- `snapshots` (id, user_id, device_id, snapshot_seq, state_json, created_at)

### Supabase Backend Schema (Postgres)

Same logical structure as SQLite but with UUID types, JSONB payload, and RLS policies. Includes a `devices` table (not yet in SQLite).

---

## Dependencies & Relationships

```
todomrs-tui ──┬── todomrs-store ──┬── todomrs-core
              │                   └── todomrs-sync ──┬── todomrs-core
              ├── todomrs-core
              ├── todomrs-sync
              ├── ratatui 0.26
              ├── crossterm 0.27
              ├── tokio (full)
              └── clap 4.0
```

### External Crate Versions
- `ratatui` 0.26, `crossterm` 0.27
- `sqlx` 0.7 (sqlite, runtime-tokio-rustls)
- `libsqlite3-sys` 0.27 (bundled)
- `tokio` 1.0 (full)
- `reqwest` 0.11 (json)
- `serde` 1.0, `serde_json` 1.0
- `chrono` 0.4 (serde)
- `uuid` 1.0 (v4, serde)
- `thiserror` 1.0, `anyhow` 1.0

---

## Start Here

**First file to open:** `/home/mp/Projects/TodoRS/crates/todomrs-tui/src/app.rs`

This is where all the application logic lives:
- The `App` struct (1537+ lines) is the central state machine
- Views, input handling, CRUD operations, sync logic, and recurrence spawning
- All the event handling that connects user keystrokes to store operations and sync

From there, trace outward:
- `main.rs` → terminal setup and event loop
- `ui.rs` → how things are rendered
- `config.rs` → how Supabase credentials are loaded
- `task_store.rs` / `operation_store.rs` → data persistence
- `client.rs` → Supabase API communication
- `operations.rs` → the sync protocol data model

---

## Constraints, Risks & Open Questions

### Constraints
- **SQLite only locally** — sync must go through operations, not DB file sync
- **Supabase free tier** — eventual portability needed; start free, move later
- **No tag sync yet** — apply_remote_operation() skips Tag operations (commented as TODO)
- **`wait!` behavior not enforced** — stored in DB but doesn't suppress next instance spawning
- **No PWA exists yet** — Phase 9 is planned but not started
- **No reminders implemented** — Phase 10

### Risks
- **Seq tracking in-memory only** — `last_synced_at` (timestamp) resets on restart; idempotent apply mitigates duplicate download, but inefficient
- **No conflict resolution** — deterministic latest-write-wins for same-field conflicts
- **Sync client timeout** — 15s hardcoded in SyncClient
- **Config stores password in plaintext** — file permissions restricted to 0o600
- **Single binary** — TUI, daemon, and eventual CLI helpers all in one binary?

### Open Questions
- Is there a `pwa/` or SvelteKit directory started anywhere? (Not found in file listing)
- What's the plan for the todo.txt import/export mentioned in Phase 10?
- Are there integration tests for the end-to-end sync flow?
- What's the deployment plan for the Supabase edge functions?
- Is there a CI/CD setup?

### Next Implementation Target
Phase 9: **PWA Mobile Client** — SvelteKit project with Supabase auth, IndexedDB cache, and operation-based sync. The plan is in `/home/mp/Projects/TodoRS/plans/phase_09.md`.
