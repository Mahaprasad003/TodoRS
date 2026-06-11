# Code Context — TodoRS Phase 6.5

## 1. Project Structure

TodoRS is a Rust workspace with 4 crates:

```
TodoRS/
├── Cargo.toml              # Workspace root, resolver = "2"
├── migrations/             # SQLite migrations (3 files)
│   ├── 0001_init.sql        # Core tables: users, projects, tags, tasks, recurrence_rules, reminders, task_tags
│   ├── 0002_operations.sql  # operations, sync_state tables
│   └── 0003_snapshots.sql   # snapshots table
├── crates/
│   ├── todomrs-core/        # Domain models + parser + recurrence engine
│   ├── todomrs-store/       # SQLite stores (TaskStore, ProjectStore, TagStore, OperationStore)
│   ├── todomrs-sync/        # Sync protocol types (Operation, Snapshot) + helper constructors
│   └── todomrs-tui/         # TUI binary (app.rs, ui.rs, main.rs)
└── plans/                   # Phase plans (phase_01.md through phase_10.md)
    └── phase_065.md          # Phase 6.5 plan: TUI polish (the target of this work)
```

### Crate Dependency Graph
```
todomrs-core  ←  todomrs-sync  ←  todomrs-store  ←  todomrs-tui
                     ↕ (no dep on store)
todomrs-core  →  todomrs-sync (depends on core for domain types)
todomrs-store →  todomrs-sync (depends on sync for Operation/Snapshot)
```

---

## 2. Domain Model (`crates/todomrs-core/src/domain.rs`)

### Task (lines 7-21)
```rust
pub struct Task {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,          // Pending | Completed
    pub project_id: Option<Uuid>,
    pub tag_ids: Vec<Uuid>,
    pub priority: Priority,          // None | Low | Medium | High | Urgent
    pub due_at: Option<DateTime<Utc>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub recurrence_rule_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

**Key methods:**
- `Task::new(user_id, title)` — creates Pending task with current timestamps
- `task.complete()` — sets status=Completed, sets completed_at, no-ops if already completed or deleted
- `task.uncomplete()` — reverts to Pending, clears completed_at
- `task.delete()` — sets deleted_at
- `task.is_deleted()` — checks deleted_at.is_some()
- `task.is_overdue()` — true if Pending + due_at < Utc::now() + not deleted

### Project (lines 41-50)
```rust
pub struct Project {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}
```
**Note:** No `Project::new()` constructor exists. Tests create projects manually. Phase 6.5 will need to build projects inline or add a constructor.

### Tag (lines 52-60), Reminder (lines 62-73), RecurrenceRule (lines 75-86)
All defined with full fields. RecurrenceRule has `kind: RecurrenceKind` (Daily/Weekly/Monthly/Yearly) and `interval: i32`.

---

## 3. Parser (`crates/todomrs-core/src/parser.rs`)

### `ParsedTask` struct (lines 8-18)
```rust
pub struct ParsedTask {
    pub title: String,
    pub project: Option<String>,     // Extracted from +project
    pub tags: Vec<String>,           // Extracted from @tag
    pub priority: Priority,          // p1-p4
    pub due_date: Option<String>,    // from due: prefix or raw date words
    pub due_time: Option<String>,    // 8pm, 9am, 14:30
    pub recurrence: Option<String>,  // every day, every 2 weeks
}
```

### `NaturalLanguageParser` (lines 21-135)
- `NaturalLanguageParser::parse(input)` — returns `ParsedTask`
- `NaturalLanguageParser::create_task_from_input(input, user_id)` — returns `(Task, Option<RecurrenceRule>)`

**CRITICAL for Phase 6.5:** `create_task_from_input` does NOT expose `parsed.project`. It only uses parsed title, priority, due_at, and recurrence to build the Task+RecurrenceRule. The project field is completely ignored. Phase 6.5 Task 2 requires extracting `+project` from parsed input — the plan recommends modifying the return type to include `Option<String>` for the project name.

### `ParsedTask` resolution methods:
- `resolve_date()` → `Option<NaiveDate>` — today, tomorrow, weekday names
- `resolve_time()` → `Option<NaiveTime>` — 8pm, 14:30
- `resolve_datetime()` → `Option<DateTime<Utc>>` — combines date+time

### `next_weekday` bug (lines 280-287)
**Current bug:** `+ 1` at end always adds at least 1 day, so if today is Friday and you say `friday`, it returns next Friday (7 days) but if today is Thursday, `friday` returns +1 day (tomorrow). The issue is the `+ 1` means it can never return the same day when the target is 0 days ahead. The `+ 1` coupled with `+ 6` in modulo means:
- Mon→Mon: (0-0+6)%7+1 = 7 ✓ (next week)
- Thu→Fri: (4-3+6)%7+1 = 1 ✓ (1 day ahead)
- But the existing test `test_next_weekday_same_day_is_next_week` already passes with the current code because it tests Thu→Thu which is 7 days.

The fix needed per the plan: if target_day == current_day, return +7 days; otherwise calculate days_ahead as (target_day - current_day + 7) % 7 (0-6 range).

---

## 4. Store Layer (`crates/todomrs-store/src/`)

### TaskStore (task_store.rs)
- `TaskStore::new(pool)` — constructor
- `create(&self, task)` — INSERT with tag associations in tx
- `get_by_id(&self, id)` — returns `Option<Task>` with tag_ids loaded
- `get_all(&self, user_id)` — returns `Vec<Task>` excluding deleted, with tags batch-loaded
- `update(&self, task)` — UPDATE all fields + replace tags
- `soft_delete(&self, id)` — sets deleted_at/updated_at
- `hard_delete(&self, id)` — DELETE row

### ProjectStore (project_store.rs)
- `ProjectStore::new(pool)`
- `create(&self, project)`
- `get_by_id(&self, id)` → `Option<Project>`
- `get_all(&self, user_id)` — excludes archived, ordered by sort_order
- `update(&self, project)`
- `soft_delete(&self, id)` — sets archived_at
- `hard_delete(&self, id)`

**⚠ MISSING:** There is **NO `find_by_name` method** on `ProjectStore`. The Phase 6.5 plan references `self.project_store.find_by_name(self.user_id, &project_name)` as if it exists — this method must be added.

### OperationStore (operation_store.rs)
- `append(&self, op)` — INSERT operation row
- `get_unsynced(&self, user_id)` — operations where synced_at IS NULL
- `mark_synced(&self, op_ids)` — sets synced_at
- `get_next_seq(&self, user_id, device_id)` — MAX(seq)+1
- `create_snapshot(&self, user_id, device_id, seq, tasks, projects, tags)`
- `get_latest_snapshot(&self, user_id)` → `Option<Snapshot>`

### TagStore (tag_store.rs)
- Standard CRUD: `create`, `get_by_id`, `get_all`, `update`, `hard_delete`

### Database (db.rs)
- Wraps `SqlitePool` with pragmas: `foreign_keys = ON`, `journal_mode = WAL`
- `new(database_url)` — connects with `SqlitePoolOptions`
- `pool()` — returns `&SqlitePool`

---

## 5. Sync Layer (`crates/todomrs-sync/src/`)

### Operation (operations.rs)
```rust
pub struct Operation {
    pub op_id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub seq: i64,
    pub entity: Entity,         // Task | Project | Tag | Reminder | RecurrenceRule
    pub entity_id: Uuid,
    pub op_type: OperationType, // Create | Update | Delete
    pub payload: OperationPayload,
    pub created_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
}
```

**Helper constructors already exist:**
- `Operation::create_task(user_id, device_id, seq, &task)` — creates TaskCreate payload
- `Operation::complete_task(user_id, device_id, seq, task_id)` — creates TaskUpdate with Completed status
- `Operation::update_task_title(user_id, device_id, seq, task_id, new_title)` — creates TaskUpdate with new title

**Note:** The `OperationPayload::TaskUpdate` variant has `completed_at: Option<DateTime<Utc>>` field. The app.rs `toggle_complete` builds operations manually inline rather than using `Operation::complete_task` — it constructs the full struct directly. The Phase 6.5 plan does the same pattern for delete/edit operations but doesn't use existing helpers.

### Snapshot (snapshot.rs)
- Contains user_id, device_id, snapshot_seq, tasks, projects, tags, created_at

---

## 6. TUI (`crates/todomrs-tui/src/`)

### app.rs — Current State

**View enum (line 8-13):**
```rust
pub enum View {
    Inbox,
    Today,
    Upcoming,
    Projects,
}
// Missing: Completed
```

**InputMode enum (line 15-18):**
```rust
pub enum InputMode {
    Normal,
    Editing,
}
// Missing: EditingTask, Searching
```

**App struct (lines 20-35):**
```rust
pub struct App {
    pub should_quit: bool,
    pub current_view: View,
    pub selected_index: usize,
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub show_help: bool,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub task_store: TaskStore,
    pub op_store: OperationStore,
    pub status_message: Option<String>,
}
// Missing: project_store, search_query, cursor_position, project_counts, previous_view
```

**App::new signature (line 63):**
```rust
pub fn new(user_id, device_id, task_store, op_store) -> Self
// Missing: project_store parameter
```

**Methods:**
- `refresh_tasks()` — loads tasks from db, clamps selection
- `filtered_tasks()` — filters by view (Inbox=all, Today=due today, Upcoming=due after today, Projects=empty)
- `handle_event(event)` — dispatches on InputMode
- `create_task_from_input()` — parses input, creates task+op, persists
- `toggle_complete()` — toggles selected task, creates op
- `delete_task()` — soft-deletes selected task, creates op
- `next_item()`, `previous_item()`

**Key bindings currently implemented:**
- `1-4` — switch views
- `j/k` or Up/Down — navigation
- `a` — start add task input
- `x` — toggle complete
- `d` — delete task
- `?` — help
- `q` — quit
- `Enter/Esc` in editing mode — confirm/cancel

**Missing for Phase 6.5:**
- `e` — edit task
- `/` — search
- `5` — completed view
- `C` — clear completed
- Arrow keys/Ctrl+A/Ctrl+E/Ctrl+W in input modes
- `project_store` field and injection

### ui.rs — Current State

**Layout:**
```
┌──────────────────────────────────────┐
│  Sidebar (20%)  │  Main Content (80%) │
│  ┌──────────┐   │  ┌──────────────┐   │
│  │ Views     │   │  │ Inbox/Today/ │   │
│  │ - Inbox   │   │  │ Upcoming/etc │   │
│  │ - Today   │   │  │             │   │
│  │ - Upcoming│   │  │ (task list) │   │
│  │ - Projects│   │  │             │   │
│  └──────────┘   │  └──────────────┘   │
├──────────────────────────────────────┤
│  Input field (3 lines)               │
├──────────────────────────────────────┤
│  Status bar (1 line)                 │
└──────────────────────────────────────┘
```

**Functions:**
- `draw(f, app)` — main layout
- `draw_sidebar(f, app, area)` — list of views
- `draw_main_content(f, app, area)` — task list with priority/status/due formatting
- `draw_input_field(f, app, area)` — input prompt + cursor + status message
- `draw_status_bar(f, app, area)` — bottom bar with shortcuts
- `draw_help(f)` — centered overlay

**Rendering details:**
- Completed tasks rendered with CROSSED_OUT modifier + DarkGray
- Overdue tasks rendered in Red
- Priority indicators: `!!! ` (Urgent), `!! ` (High), `! ` (Medium)
- Status icons: `✓ ` (Completed), `⚠ ` (Overdue), `□ ` (Pending)
- Due dates: `dd/mm HH:MM` format, only show time if non-midnight
- Cursor only set in Editing mode (no cursor_position tracking)

### main.rs — Current State

- Loads persistent device IDs from `.todomrs_user_id` and `.todomrs_device_id` files
- Database: `sqlite://./todomrs.db?mode=rwc`
- Runs migrations on startup via `sqlx::migrate!("../../migrations")`
- Creates `TaskStore` and `OperationStore`
- Creates local user with `INSERT OR IGNORE INTO users`
- Panic hook restores terminal
- 100ms poll interval

**Missing for Phase 6.5:**
- `ProjectStore` initialization (needed for +project auto-create and sidebar)
- Pass `project_store` to `App::new`

---

## 7. Database Schema (`migrations/`)

### Users
```sql
CREATE TABLE users (id BLOB PRIMARY KEY, email TEXT UNIQUE, created_at TEXT, updated_at TEXT);
```

### Projects
```sql
CREATE TABLE projects (
    id BLOB PRIMARY KEY, user_id BLOB REFERENCES users(id),
    name TEXT, color TEXT, sort_order INTEGER DEFAULT 0,
    created_at TEXT, updated_at TEXT, archived_at TEXT
);
```

### Tags
```sql
CREATE TABLE tags (
    id BLOB PRIMARY KEY, user_id BLOB REFERENCES users(id),
    name TEXT, color TEXT, created_at TEXT, updated_at TEXT
);
```

### Tasks
```sql
CREATE TABLE tasks (
    id BLOB PRIMARY KEY, user_id BLOB REFERENCES users(id),
    title TEXT, description TEXT, status TEXT DEFAULT 'pending',
    project_id BLOB REFERENCES projects(id) ON DELETE SET NULL,
    priority TEXT DEFAULT 'none', due_at TEXT, scheduled_at TEXT,
    recurrence_rule_id BLOB, created_at TEXT, updated_at TEXT,
    completed_at TEXT, deleted_at TEXT
);
```

### task_tags (junction)
```sql
CREATE TABLE task_tags (task_id BLOB REFERENCES tasks(id), tag_id BLOB REFERENCES tags(id), PRIMARY KEY (task_id, tag_id));
```

### recurrence_rules, reminders
```sql
CREATE TABLE recurrence_rules (id BLOB PK, task_id BLOB REFERENCES tasks(id), kind TEXT, interval INTEGER, ...);
CREATE TABLE reminders (id BLOB PK, task_id BLOB REFERENCES tasks(id), remind_at TEXT, status TEXT DEFAULT 'pending', ...);
```

### Operations (for sync)
```sql
CREATE TABLE operations (
    op_id TEXT PRIMARY KEY, user_id TEXT, device_id TEXT, seq INTEGER,
    entity TEXT, entity_id TEXT, op_type TEXT, payload TEXT,
    created_at TEXT, synced_at TEXT
);
CREATE TABLE sync_state (user_id TEXT PK, device_id TEXT, last_local_seq INTEGER, ...);
CREATE TABLE snapshots (id INTEGER PK AUTOINCREMENT, user_id TEXT, device_id TEXT, snapshot_seq INTEGER, state_json TEXT, created_at TEXT);
```

---

## 8. Phase 6.5 Goals — Specific Changes Needed

Based on `plans/phase_065.md`, the following 8 tasks are required:

### Task 1: Fix Weekday Resolution Bug
- **File:** `crates/todomrs-core/src/parser.rs`
- **Change:** `next_weekday()` function. Remove the `+ 1` logic. If target_day == current_day, return +7 days. Otherwise compute `(target_day - current_day + 7) % 7`.
- **Existing tests that must still pass:** `test_next_weekday_same_day_is_next_week`, `test_next_weekday_next_day`, `test_next_weekday_wrap_around`

### Task 2: Wire Up +project Auto-Create
- **Files:** `crates/todomrs-tui/src/app.rs`, `crates/todomrs-tui/src/main.rs`
- **Changes:**
  1. Add `ProjectStore` field to `App`, inject in constructor and `new()` signature
  2. Initialize `ProjectStore` in `main.rs`, pass to `App::new`
  3. Modify `create_task_from_input` to extract `+project` from input
  4. Look up or create project via `ProjectStore`, set `task.project_id`
  5. **Need to add `ProjectStore::find_by_name` method** (doesn't exist yet)
  6. **Need to modify parser** to expose `parsed.project` from `create_task_from_input` (return `(Task, Option<RecurrenceRule>, Option<String>)`)
- **Alternative:** Extract `+project` from `input_buffer` directly in app.rs using simple string operations (avoids parser change but duplicates logic)

### Task 3: Edit Task (e key)
- **File:** `crates/todomrs-tui/src/app.rs`
- **Changes:**
  1. Add `InputMode::EditingTask(usize)` variant
  2. Bind `e` key: set input_buffer to selected task title, enter EditingTask mode
  3. Handle Enter in EditingTask: call `update_task_title`, record Operation::update_task_title
  4. Esc cancels
  5. Update UI to show different prompt
  6. Need `update_task_title` async method on App

### Task 4: Search Functionality (/ key)
- **File:** `crates/todomrs-tui/src/app.rs`
- **Changes:**
  1. Add `InputMode::Searching` variant
  2. Add `search_query: String` and `previous_view: Option<View>` fields
  3. Bind `/` key: save current view, enter Searching mode
  4. Filter `filtered_tasks()` by `search_query` substring match (case-insensitive)
  5. Enter confirms search (stays filtered), Esc clears and restores view
  6. UI shows search indicator

### Task 5: Completed/Archive View
- **Files:** `crates/todomrs-tui/src/app.rs`, `crates/todomrs-tui/src/ui.rs`
- **Changes:**
  1. Add `View::Completed` variant
  2. In `filtered_tasks` — return tasks where status == Completed and not deleted
  3. Bind `5` key to switch to Completed view
  4. Bind `C` key to clear all completed (soft-delete each)
  5. Sidebar update: add Completed item at index 4
  6. Main content title: "Completed"
  7. Status bar: add `C` shortcut

### Task 6: Input Field Navigation
- **File:** `crates/todomrs-tui/src/app.rs`
- **Changes:**
  1. Add `cursor_position: usize` field
  2. Update Char to insert at cursor position
  3. Update Backspace to remove at cursor position
  4. Add Left/Right arrow navigation
  5. Add Ctrl+A (Home), Ctrl+E (End), Ctrl+W (delete word backwards)
  6. UI: set cursor based on cursor_position
  7. Reset cursor position on mode entry

### Task 7: Sidebar Project Display
- **Files:** `crates/todomrs-tui/src/app.rs`, `crates/todomrs-tui/src/ui.rs`
- **Changes:**
  1. Add `project_counts: HashMap<Uuid, (String, usize)>` field
  2. Add `refresh_project_counts` method
  3. Call from `refresh_tasks`
  4. Sidebar: add "Projects" section showing project names with counts

### Task 8: Update Help Text
- **File:** `crates/todomrs-tui/src/ui.rs`
- **Changes:**
  1. Update `draw_help` to include all new shortcuts (e, /, 5, C, arrows, Ctrl+A/E/W)

---

## 9. Key Risks & Open Questions

1. **`ProjectStore::find_by_name` doesn't exist** — must be added. Signature: `async fn find_by_name(&self, user_id: Uuid, name: &str) -> Result<Option<Project>>` with a `WHERE user_id = ? AND name = ?` query.

2. **Parser `create_task_from_input` doesn't expose project** — return type needs modification from `(Task, Option<RecurrenceRule>)` to `(Task, Option<RecurrenceRule>, Option<String>)`. This changes the call site in `create_task_from_input` in app.rs.

3. **No `Project::new` constructor exists** — projects are built manually in tests. Phase 6.5 Task 2 needs `Project::new(user_id, name)` or inline construction.

4. **EditingTask uses filtered task index** — the `EditingTask(usize)` variant indexes into `filtered_tasks()` which is a derived slice. If tasks change between entering edit mode and saving, the index could be stale. The current approach in the plan uses the index from `filtered_tasks()` directly.

5. **Cursor position for search** — the plan notes a `search_cursor_position` field may be needed but wouldn't be separate if sharing the same `cursor_position` field for all input modes.

6. **View::Completed filtering** — the plan uses `t.status == TaskStatus::Completed && t.status != TaskStatus::Deleted`. But `get_all` already excludes deleted tasks (`deleted_at IS NULL`), so the deleted check is redundant. However, soft-deleted tasks might still be in `self.tasks` if they were loaded before deletion. The filtering should check `t.deleted_at.is_none()` as well.

7. **Operation::complete_task helper** — exists in sync crate but `toggle_complete` in app.rs doesn't use it. It builds operations inline. For Phase 6.5, consider using the helper for consistency.

8. **Test infrastructure** — store tests use `SqlitePool::connect("sqlite::memory:")` with `sqlx::migrate!("../../migrations")` (relative to the store crate). Any new tests would follow this pattern.

---

## 10. Files That Will Need Changes

| File | Changes Needed |
|------|---------------|
| `crates/todomrs-core/src/parser.rs` | Fix `next_weekday()` (Task 1); modify `create_task_from_input` return type to include project (Task 2) |
| `crates/todomrs-store/src/project_store.rs` | Add `find_by_name(user_id, name)` method |
| `crates/todomrs-tui/src/app.rs` | Add `ProjectStore`, `cursor_position`, `search_query`, `project_counts`, `previous_view` fields; add `EditingTask`, `Searching`, `Completed` variants; add `e`, `/`, `5`, `C` bindings; add input navigation arrows/Ctrl shortcuts; modify `create_task_from_input` for +project; add `update_task_title`, `clear_completed`, `refresh_project_counts` methods |
| `crates/todomrs-tui/src/ui.rs` | Update sidebar (Completed, project list), main content (search indicator, Completed title), input field (cursor position, new prompts), status bar (C shortcut), help text (all new shortcuts) |
| `crates/todomrs-tui/src/main.rs` | Initialize `ProjectStore`, pass to `App::new` |
| `crates/todomrs-core/src/lib.rs` | (Possibly) re-export updated parser return type |
| Possibly `crates/todomrs-core/src/domain.rs` | Add `Project::new()` constructor |

## 11. Recommended Reading Order for Implementer

1. `crates/todomrs-core/src/parser.rs` (Task 1 — weekday fix, simplest starting point)
2. `crates/todomrs-tui/src/app.rs` (core state machine — most changes needed)
3. `crates/todomrs-tui/src/ui.rs` (all UI changes)
4. `crates/todomrs-tui/src/main.rs` (ProjectStore injection)
5. `crates/todomrs-store/src/project_store.rs` (find_by_name addition)
6. `plans/phase_065.md` (full plan with code snippets)

## 12. Existing Test Coverage

- **Core:** 7 parser tests, 7 recurrence tests, 9 domain tests
- **Store:** task_store_test (CRUD, soft-delete, tag loading, update), project_store_test (CRUD, get_all ordering), operation_store_test (append, mark_synced, sequencing, snapshots)
- **Sync:** operation tests (create_task, complete_task, update_task_title)

All tests use in-memory SQLite with migrations applied. Pattern: `setup() -> (pool, store)` then seed user.
