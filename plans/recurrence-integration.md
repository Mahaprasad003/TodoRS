# Recurrence Integration — Implementation Plan

## Overview

Transform TodoRS's recurrence system from a broken stub into a fully functional, SuperProductivity-inspired recurrence engine. Currently, recurrence rules are parsed but immediately discarded, never persisted, and never trigger next-instance creation. This plan implements independent instance spawning with a dedicated Recurring view, UI indicators, proper edit preservation, and sync integration.

**Core model**: One `RecurrenceRule` config stored in `recurrence_rules` table → many independent task instances. Completing an instance spawns the next occurrence. The `recurrence_rules` HashMap is keyed by `rule.id` (the rule UUID), not by `task_id`, so spawned instances can still look up their rule via `task.recurrence_rule_id`.

---

## Current State

### What Works
- **Parser** (`crates/todomrs-core/src/parser.rs:97-143`): Correctly parses `every day/week/month/year [N]` patterns
- **Domain model** (`crates/todomrs-core/src/domain.rs:75-86`): `RecurrenceRule` struct with kind/interval/timezone
- **RecurrenceEngine** (`crates/todomrs-core/src/recurrence.rs:11-26`): `next_occurrence()` computes next date
- **Database schema** (`migrations/0001_init.sql:59-68`): `recurrence_rules` table exists with FK to tasks
- **Entity::RecurrenceRule** already exists in `crates/todomrs-sync/src/operations.rs`
- **TaskStore** already reads/writes `recurrence_rule_id` on tasks

### What's Broken
- **app.rs:519**: `let (mut task, _recurrence_rule) = ...` — rule is discarded with underscore prefix
- **No RecurrenceRuleStore**: Zero CRUD operations for `recurrence_rules` table
- **No completion → next instance**: Completing a recurring task does nothing special
- **No UI indicators**: No way to identify recurring tasks
- **No Recurring view**: Can't see all recurring tasks in one place
- **Edit strips recurrence**: `task_to_edit_string()` doesn't include recurrence pattern, and `update_task_from_input()` never persists recurrence changes

---

## Implementation Phases

### Phase 1: Schema Migration & Domain Updates

**Goal**: Add `wait_for_completion` and `anchor_mode` columns to support SuperProductivity-style recurrence.

#### 1.1 Create Migration File

**File**: `migrations/0004_recurrence_enhancements.sql`

```sql
-- Add wait_for_completion flag to recurrence_rules
ALTER TABLE recurrence_rules ADD COLUMN wait_for_completion INTEGER NOT NULL DEFAULT 0;

-- Add anchor_mode: 'schedule' (default) or 'completion'
ALTER TABLE recurrence_rules ADD COLUMN anchor_mode TEXT NOT NULL DEFAULT 'schedule';
```

Note: SQLite `ALTER TABLE ADD COLUMN` is fully supported and non-destructive. Existing rows get the DEFAULT values.

#### 1.2 Update RecurrenceRule Domain Model

**File**: `crates/todomrs-core/src/domain.rs`

**Changes**:
- Add fields to `RecurrenceRule` struct (around line 75-86):
  ```rust
  pub struct RecurrenceRule {
      pub id: Uuid,
      pub task_id: Uuid,
      pub kind: RecurrenceKind,
      pub interval: i32,
      pub by_weekday: Option<Vec<i32>>,
      pub by_monthday: Option<Vec<i32>>,
      pub timezone: String,
      pub wait_for_completion: bool,  // NEW
      pub anchor_mode: AnchorMode,     // NEW
      pub created_at: DateTime<Utc>,
      pub updated_at: DateTime<Utc>,
  }
  ```

- Add `AnchorMode` enum after `RecurrenceKind` (around line 93):
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
  #[serde(rename_all = "snake_case")]
  pub enum AnchorMode {
      Schedule,      // Always advance from original schedule date
      Completion,    // Advance from when task was completed
  }
  ```

- Update `RecurrenceEngine::create_daily_rule` and `create_weekly_rule` (lines 29-58) to set defaults:
  ```rust
  wait_for_completion: false,
  anchor_mode: AnchorMode::Schedule,
  ```

**Tests**: Add `test_recurrence_rule_serde_roundtrip` to `domain.rs` tests:
```rust
#[test]
fn test_recurrence_rule_serde_roundtrip() {
    let rule = RecurrenceRule {
        id: Uuid::new_v4(),
        task_id: Uuid::new_v4(),
        kind: RecurrenceKind::Daily,
        interval: 2,
        by_weekday: None,
        by_monthday: None,
        timezone: "UTC".to_string(),
        wait_for_completion: true,
        anchor_mode: AnchorMode::Completion,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let json = serde_json::to_string(&rule).expect("serialize");
    let deserialized: RecurrenceRule = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(rule, deserialized);
}
```

---

### Phase 2: RecurrenceRuleStore

**Goal**: Create CRUD operations for the `recurrence_rules` table.

#### 2.1 Create RecurrenceRuleStore

**New File**: `crates/todomrs-store/src/recurrence_store.rs`

**Structure** (matching existing `TaskStore` / `ProjectStore` patterns):
```rust
pub struct RecurrenceRuleStore {
    pool: SqlitePool,
}

impl RecurrenceRuleStore {
    pub fn new(pool: SqlitePool) -> Self;
    pub async fn create(&self, rule: &RecurrenceRule) -> Result<()>;
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<RecurrenceRule>>;
    pub async fn get_all(&self) -> Result<Vec<RecurrenceRule>>;
    pub async fn find_by_task_id(&self, task_id: Uuid) -> Result<Option<RecurrenceRule>>;
    pub async fn update(&self, rule: &RecurrenceRule) -> Result<()>;
    pub async fn delete(&self, id: Uuid) -> Result<()>;
}
```

**Key Implementation Details**:
- `create`: INSERT with all fields. Use `serialize_enum` helpers (reuse existing pattern from task_store.rs) for kind/anchor_mode. Store wait_for_completion as i32 (0/1).
- `get_by_id`: Simple `SELECT * FROM recurrence_rules WHERE id = ?`
- `get_all`: `SELECT * FROM recurrence_rules` (loads all rules)
- `find_by_task_id`: `SELECT * FROM recurrence_rules WHERE task_id = ?`
- `update`: UPDATE all fields
- `delete`: `DELETE FROM recurrence_rules WHERE id = ?`

**Row Mapping** (internal `RecurrenceRuleRow` struct):
```rust
#[derive(sqlx::FromRow)]
struct RecurrenceRuleRow {
    id: Uuid,
    task_id: Uuid,
    kind: String,
    interval: i32,
    by_weekday: Option<String>,   // "1,3,5" comma-separated → parse to Vec<i32>
    by_monthday: Option<String>,  // "1,15" comma-separated → parse to Vec<i32>
    timezone: String,
    wait_for_completion: i32,     // 0/1 → bool
    anchor_mode: String,          // "schedule" or "completion"
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

Parse helpers for Vec<i32> (needed because sqlx doesn't natively support Vec<i32> with SQLite):
```rust
fn parse_comma_separated_i32(s: Option<String>) -> Option<Vec<i32>> {
    s.filter(|s| !s.is_empty()).map(|s| {
        s.split(',')
            .filter_map(|part| part.trim().parse::<i32>().ok())
            .collect()
    })
}

fn format_comma_separated_i32(v: &Option<Vec<i32>>) -> Option<String> {
    v.as_ref().map(|v| v.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","))
}
```

#### 2.2 Export RecurrenceRuleStore

**File**: `crates/todomrs-store/src/lib.rs`

**Changes**:
- Add `pub mod recurrence_store;`
- Add `pub use recurrence_store::RecurrenceRuleStore;`

#### 2.3 Test Pattern

**Use existing `setup_pool()` pattern** from `crates/todomrs-store/tests/` (not inline helpers):
```rust
#[tokio::test]
async fn test_create_recurrence_rule() {
    let pool = test_helpers::setup_pool().await;
    let store = RecurrenceRuleStore::new(pool);
    
    let task_id = Uuid::new_v4();
    let rule = RecurrenceRule { ... };
    store.create(&rule).await.unwrap();
    
    let loaded = store.find_by_task_id(task_id).await.unwrap().unwrap();
    assert_eq!(loaded.kind, RecurrenceKind::Daily);
    assert_eq!(loaded.interval, 1);
}
```

---

### Phase 3: Persist Recurrence Rules on Create

**Goal**: Fix app.rs to persist the RecurrenceRule instead of discarding it.

#### 3.1 Inject RecurrenceRuleStore into App

**File**: `crates/todomrs-tui/src/main.rs`

**Changes**:
- Add `RecurrenceRuleStore` to imports from `todomrs_store`
- Create the store: `let recurrence_store = RecurrenceRuleStore::new(db.pool().clone());`
- Pass to `App::new()`

#### 3.2 Add RecurrenceRuleStore to App Struct

**File**: `crates/todomrs-tui/src/app.rs`

**Changes**:
- Add `pub recurrence_store: RecurrenceRuleStore` field to `App` struct
- Add `use std::collections::HashMap;` import (not yet imported)
- Add `pub recurrence_rules: HashMap<Uuid, RecurrenceRule>` — **keyed by `rule.id`** (not task_id), so spawned instances can look up their rule via `task.recurrence_rule_id`
- Update `App::new()` signature and initialization

#### 3.3 Fix create_task_from_input to Persist Rule

**File**: `crates/todomrs-tui/src/app.rs`

**Before** (line ~574):
```rust
let (mut task, _recurrence_rule) =
    NaturalLanguageParser::create_task_from_input(&input, self.user_id);
```

**After**:
```rust
let (mut task, recurrence_rule) =
    NaturalLanguageParser::create_task_from_input(&input, self.user_id);

// Persist recurrence rule if present
if let Some(rule) = &recurrence_rule {
    self.recurrence_store.create(rule).await?;
    // Record operation for sync
    let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
    let op = Operation::create_recurrence_rule(self.user_id, self.device_id, seq, rule);
    self.op_store.append(&op).await?;
}
```

**IMPORTANT — wait!/every! prefix handling in create_task_from_input**:

The existing `NaturalLanguageParser::create_task_from_input()` internally calls `Self::parse()` and then inspects `parsed.recurrence` to build the rule. But `parsed.recurrence` will contain `"wait! every day"` or `"every! day"` — the current rule builder does `words[0].eq_ignore_ascii_case("every")` which will fail.

**Fix**: Add new fields to `ParsedTask` struct:
```rust
pub struct ParsedTask {
    pub title: String,
    pub project: Option<String>,
    pub tags: Vec<String>,
    pub priority: Priority,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub recurrence: Option<String>,
    // NEW:
    pub wait_for_completion: bool,
    pub anchor_mode: AnchorMode,
}
```

Update `NaturalLanguageParser::parse()` to:
1. Detect `wait!` prefix before `every` → set `wait_for_completion: true`, strip it from the recurrence string
2. Detect `every!` (instead of `every`) → set `anchor_mode: Completion`, but store `"every"` (without `!`) in the recurrence string so existing parsing still works
3. Both can be combined: `wait! every! 3 days` → `wait_for_completion: true, anchor_mode: Completion, recurrence: "every 3 days"`

Update `create_task_from_input()` to read `parsed.wait_for_completion` and `parsed.anchor_mode` and pass them to the `RecurrenceRule` creation.

#### 3.4 Update refresh_tasks to Load Rules

**File**: `crates/todomrs-tui/src/app.rs`

**Changes in `refresh_tasks`**:
```rust
pub async fn refresh_tasks(&mut self) -> Result<()> {
    self.tasks = self.task_store.get_all(self.user_id).await?;
    self.refresh_project_counts().await?;
    
    // Load all recurrence rules — KEYED BY rule.id, not task_id
    let rules = self.recurrence_store.get_all().await?;
    self.recurrence_rules = rules.into_iter().map(|r| (r.id, r)).collect();
    
    // Clamp selection
    let count = self.filtered_tasks().len();
    if count > 0 && self.selected_index >= count {
        self.selected_index = count.saturating_sub(1);
    }
    Ok(())
}
```

---

### Phase 4: Completion → Next Instance Spawning

**Goal**: When completing a recurring task, automatically create the next instance.

#### 4.1 Update toggle_complete Logic

**File**: `crates/todomrs-tui/src/app.rs`

After the existing completion/uncompletion logic, add:
```rust
// Handle recurrence: when completing (not uncompleting) a recurring task
if !completed {
    // Look up by task.recurrence_rule_id (not task.id!)
    if let Some(rule_id) = task.recurrence_rule_id {
        if let Some(rule) = self.recurrence_rules.get(&rule_id).cloned() {
            self.spawn_next_recurrence(&task, &rule).await?;
            return Ok(()); // refresh is handled inside spawn_next
        }
    }
}
```

#### 4.2 Implement spawn_next_recurrence

**File**: `crates/todomrs-tui/src/app.rs`

```rust
async fn spawn_next_recurrence(&mut self, completed_task: &Task, rule: &RecurrenceRule) -> Result<()> {
    // Determine anchor date for next occurrence
    let anchor = match rule.anchor_mode {
        AnchorMode::Schedule => completed_task.due_at.unwrap_or(completed_task.created_at),
        AnchorMode::Completion => completed_task.completed_at.unwrap_or_else(Utc::now),
    };

    // Compute next due date
    let next_due = RecurrenceEngine::next_occurrence(rule, anchor);

    // Create new task instance (independent copy)
    let mut new_task = Task::new(self.user_id, completed_task.title.clone());
    new_task.project_id = completed_task.project_id;
    new_task.tag_ids = completed_task.tag_ids.clone();
    new_task.priority = completed_task.priority.clone();
    new_task.due_at = Some(next_due);
    new_task.recurrence_rule_id = Some(rule.id); // links back to the same rule

    // Persist
    self.task_store.create(&new_task).await?;

    // Record operation
    let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
    let op = Operation::create_task(self.user_id, self.device_id, seq, &new_task);
    self.op_store.append(&op).await?;

    self.status_message = Some(format!(
        "Completed: {} → Next: {}",
        completed_task.title,
        format_recurrence_rule(rule)
    ));

    self.refresh_tasks().await?;
    Ok(())
}
```

#### 4.3 Format Helper

**File**: In `crates/todomrs-tui/src/app.rs` (or consider adding a `Display` impl in `domain.rs`):

```rust
/// Format a recurrence rule into a human-readable string
pub fn format_recurrence_rule(rule: &RecurrenceRule) -> String {
    let kind_str = match rule.kind {
        RecurrenceKind::Daily => "day",
        RecurrenceKind::Weekly => "week",
        RecurrenceKind::Monthly => "month",
        RecurrenceKind::Yearly => "year",
    };
    
    let base = if rule.interval == 1 {
        format!("every {}", kind_str)
    } else {
        format!("every {} {}s", rule.interval, kind_str)
    };

    let prefix = match rule.wait_for_completion {
        true => "wait! ",
        false => "",
    };
    let suffix = match rule.anchor_mode {
        AnchorMode::Completion => " (from completion)",
        AnchorMode::Schedule => "",
    };

    format!("{}{}{}", prefix, base, suffix)
}
```

---

### Phase 5: UI Indicators for Recurring Tasks

**Goal**: Show visual indicators for recurring tasks in the task list.

#### 5.1 Add Recurrence Indicator to Task Rendering

**File**: `crates/todomrs-tui/src/ui.rs`

**Changes in `draw_main_content` task rendering loop**:

```rust
// Get recurrence indicator if task has a recurrence rule
let recurrence_indicator = if let Some(rule_id) = task.recurrence_rule_id {
    if let Some(rule) = app.recurrence_rules.get(&rule_id) {
        let interval_str = if rule.interval == 1 {
            String::new()
        } else {
            format!("{}", rule.interval)
        };
        let kind_str = match rule.kind {
            todomrs_core::domain::RecurrenceKind::Daily => format!("{}d", interval_str),
            todomrs_core::domain::RecurrenceKind::Weekly => format!("{}w", interval_str),
            todomrs_core::domain::RecurrenceKind::Monthly => format!("{}m", interval_str),
            todomrs_core::domain::RecurrenceKind::Yearly => format!("{}y", interval_str),
        };
        format!("♻{} ", kind_str)
    } else {
        String::new()
    }
} else {
    String::new()
};
```

Insert into task display:
```rust
let full_text = format!(
    "{}{}{}{}",
    status_icon, priority_indicator, recurrence_indicator, task.title, suffix
);
```

**Example output**:
- `□ ♻ Write report [12/06]` (daily, interval=1)
- `□ ♻2w Review PR [25/06]` (every 2 weeks)
- `□ ♻3m Pay rent` (every 3 months, no due date yet)

**Placement rationale**: The ♻ indicator goes between priority and title, mirroring the visual "this task's behavior is special" position. If the due date is truncated on narrow terminals, the indicator is more important context than the due date.

---

### Phase 6: Recurring View

**Goal**: Add a dedicated view (key `6`) to see all recurring tasks and their rules.

#### 6.1 Add View::Recurring Variant

**File**: `crates/todomrs-tui/src/app.rs`

```rust
pub enum View {
    Inbox,
    Today,
    Upcoming,
    Projects,
    Completed,
    Recurring,  // NEW
}
```

#### 6.2 Add Keybinding for Recurring View

```rust
KeyCode::Char('6') => {
    self.current_view = View::Recurring;
    self.selected_index = 0;
    self.selected_project_id = None;
    self.previous_view = None;
}
```

#### 6.3 Update filtered_tasks

```rust
View::Recurring => self
    .tasks
    .iter()
    .filter(|t| t.recurrence_rule_id.is_some() && t.deleted_at.is_none())
    .collect(),
```

#### 6.4 Update Sidebar

**File**: `crates/todomrs-tui/src/ui.rs`

- Add "Recurring" to `view_items`
- Update `view_selected` to include `View::Recurring => 5`
- Update sidebar height from `Constraint::Length(7)` to `Constraint::Length(8)`

#### 6.5 Update Status Bar & Help

**File**: `crates/todomrs-tui/src/ui.rs`

- Add `View::Recurring => "Recurring"` to `view_name` match
- Add `Line::from("  6      — Recurring view")` to help text

---

### Phase 7: Edit Preservation of Recurrence

**Goal**: When editing a recurring task, the edit buffer includes the recurrence pattern, AND saving the edit properly creates/updates/deletes the recurrence rule.

#### 7.1 Update task_to_edit_string

**File**: `crates/todomrs-tui/src/app.rs`

**Add recurrence and prefixes to edit string**:
```rust
// Include recurrence pattern with prefixes
if let Some(rule) = self.recurrence_rules.get(&task.recurrence_rule_id?) {
    // Prefix for wait_for_completion
    if rule.wait_for_completion {
        parts.push("wait!".to_string());
    }
    
    // Prefix for anchor_mode
    let every_prefix = match rule.anchor_mode {
        AnchorMode::Completion => "every!",
        AnchorMode::Schedule => "every",
    };
    
    let kind_str = match rule.kind {
        RecurrenceKind::Daily => "day",
        RecurrenceKind::Weekly => "week",
        RecurrenceKind::Monthly => "month",
        RecurrenceKind::Yearly => "year",
    };
    
    parts.push(if rule.interval == 1 {
        format!("{} {}", every_prefix, kind_str)
    } else {
        format!("{} {} {}", every_prefix, rule.interval, kind_str)
    });
}
```

**Example outputs**:
- Task with `wait_for_completion: true, interval: 2, kind: Weekly` → edit buffer includes `wait! every 2 weeks`
- Task with `anchor_mode: Completion, interval: 1, kind: Day` → edit buffer includes `every! day`

#### 7.2 update_task_from_input — Handle Recurrence Changes

**File**: `crates/todomrs-tui/src/app.rs`

This is the critical save path when editing a task. Currently it re-parses the input and updates title/priority/due_at/project but **completely ignores recurrence**. It must be extended to:

1. After parsing the input, check if a recurrence pattern was detected
2. Get the existing rule (if any) via `task.recurrence_rule_id`
3. Three cases:
   - **New recurrence**: Task had no rule, parsed input has one → create new `RecurrenceRule`, persist to DB, create sync op
   - **Changed recurrence**: Task had a rule, parsed input has a different pattern → update the existing rule's kind/interval/anchor_mode/wait_for_completion, create sync op
   - **Removed recurrence**: Task had a rule, parsed input has none → delete the rule from DB, clear `task.recurrence_rule_id`, create sync op
   - **Unchanged**: Task had a rule, parsed input has same pattern → no change needed
4. When creating/updating a rule, also update `task.recurrence_rule_id`

**Pseudo-code**:
```rust
// After parsing the input, handle recurrence
let parsed = NaturalLanguageParser::parse(&input);
match (parsed.recurrence, task.recurrence_rule_id) {
    (Some(rec), None) => {
        // New recurrence — create rule
        let rule = create_rule_from_parsed(...);
        self.recurrence_store.create(&rule).await?;
        task.recurrence_rule_id = Some(rule.id);
    }
    (Some(rec), Some(rule_id)) => {
        // Possibly changed recurrence — check and update
        if let Some(existing) = self.recurrence_rules.get(&rule_id) {
            let new_rule = create_rule_from_parsed(...);
            if has_changed(existing, &new_rule) {
                self.recurrence_store.update(&new_rule).await?;
            }
        }
    }
    (None, Some(rule_id)) => {
        // Recurrence removed — delete rule
        self.recurrence_store.delete(rule_id).await?;
        task.recurrence_rule_id = None;
    }
    (None, None) => {} // No change
}
```

**This is essential** — without it, editing a task with any recurrence pattern will appear to work but silently leave stale/orphaned rules in the database.

---

### Phase 8: Wait-for-Completion Mode (Deferred)

**Decision**: Defer full `wait_for_completion` implementation to a follow-up phase. The schema column and domain model field are added in Phase 1 for data compatibility, but the UI logic is not implemented now.

**Rationale**: The current plan's `wait_for_completion` filter logic was equivalent to normal task visibility (just shows pending tasks). True "wait for completion" requires tracking which instance is "current" and suppressing future instances until the current one is done — this needs instance-ordering metadata that's better handled as a separate feature.

**What IS implemented in this phase:**
- Schema column exists for forward compatibility
- Parser recognizes `wait!` prefix and sets the flag on the rule
- `format_recurrence_rule` displays it in edit buffer and status messages
- The flag is persisted and round-tripped

**What is NOT implemented:**
- Suppressing next instance creation until current is completed
- UI to toggle the flag without re-editing the task

---

### Phase 9: Anchor Mode Support

**Goal**: Support both schedule-anchored and completion-anchored recurrence.

#### 9.1 Parser Support for `every!` Prefix

**File**: `crates/todomrs-core/src/parser.rs`

Add detection in `parse()`:
- When encountering `every!` (with exclamation mark), set `anchor_mode: AnchorMode::Completion`
- Strip the `!` and normalize to `"every"` in the stored `recurrence` string (so downstream parsing of `"every N days"` continues to work)

**Example**: `every! 3 days` → `parsed.anchor_mode = Completion`, `parsed.recurrence = "every 3 days"`

#### 9.2 spawn_next_recurrence Uses Anchor Mode

Already implemented in Phase 4.2 — the `anchor` variable uses `rule.anchor_mode` to decide whether to advance from the original due date or from the completion timestamp.

---

### Phase 10: Sync Integration

**Goal**: Add `OperationPayload` variants for recurrence rule operations.

#### 10.1 Add RecurrenceRuleCreate/Update Payloads

**File**: `crates/todomrs-sync/src/operations.rs`

```rust
RecurrenceRuleCreate {
    task_id: Uuid,
    kind: String,
    interval: i32,
    timezone: String,
    wait_for_completion: bool,
    anchor_mode: String,
},
RecurrenceRuleUpdate {
    interval: Option<i32>,
    wait_for_completion: Option<bool>,
    anchor_mode: Option<String>,
},
```

**Note on enum serialization**: Use `serde_json::to_value(&rule.kind).as_str().unwrap().to_string()` or the existing `serialize_enum` pattern from `task_store.rs`, NOT `format!("{:?}", ...)` which relies on the Debug representation that may change.

#### 10.2 Add Helper Constructor

```rust
pub fn create_recurrence_rule(
    user_id: Uuid,
    device_id: Uuid,
    seq: i64,
    rule: &RecurrenceRule,
) -> Self {
    Self {
        op_id: Uuid::new_v4(),
        user_id,
        device_id,
        seq,
        entity: Entity::RecurrenceRule,
        entity_id: rule.id,
        op_type: OperationType::Create,
        payload: OperationPayload::RecurrenceRuleCreate {
            task_id: rule.task_id,
            kind: serialize_enum_kind(&rule.kind),
            interval: rule.interval,
            timezone: rule.timezone.clone(),
            wait_for_completion: rule.wait_for_completion,
            anchor_mode: serialize_enum_anchor(&rule.anchor_mode),
        },
        created_at: Utc::now(),
        synced_at: None,
    }
}
```

---

### Phase 11: Testing

**Goal**: Add unit and integration tests for all recurrence features.

#### Test Scenarios

1. **RecurrenceRuleStore CRUD**: Create, read, update, delete rules
2. **Find rule by task_id**: Ensure find returns correct rule
3. **Create recurring task**: Create via parser → verify rule in DB
4. **Complete recurring task**: Complete → verify new task spawned with next due date
5. **Schedule anchor**: Complete → next due = original_schedule + interval
6. **Completion anchor**: Complete → next due = completed_at + interval
7. **wait_for_completion flag**: Verify round-trip through store
8. **Edit recurrence**: Update rule kind/interval via edit → verify DB updated
9. **Remove recurrence**: Edit task to remove "every X" → verify rule deleted from DB
10. **to_edit_string round-trip**: Recurring task → edit string → re-parse → same rule
11. **Toggle idempotency**: Uncomplete/re-complete should not spawn duplicate instances
12. **Monthly edge case**: January 31 + 1 month → February 28 (or next valid month)

---

## FK Cascade Note

The `recurrence_rules.task_id` column has `REFERENCES tasks(id) ON DELETE CASCADE`. This means if the **original** task (the one that created the rule) is hard-deleted, the rule is also deleted, which orphans all spawned instances' `recurrence_rule_id` references.

**Mitigation**: The app only soft-deletes tasks (`deleted_at IS NOT NULL`), not hard-deletes, so the cascade should never trigger during normal use. If a future hard-delete operation is added, it must also clean up or detach spawned recurrence instances.

---

## Implementation Order

```
Phase 1: Schema Migration & Domain Updates
    ↓
Phase 2: RecurrenceRuleStore
    ↓
Phase 3: Persist Recurrence Rules on Create  (includes parser fixes)
    ↓
Phase 4: Completion → Next Instance Spawning
    ↓
Phase 5: UI Indicators for Recurring Tasks
    ↓
Phase 6: Recurring View
    ↓
Phase 7: Edit Preservation (full round-trip, including save)
    ↓
Phase 9: Anchor Mode Support  (can be parallel with 5-7)
    ↓
Phase 10: Sync Integration  (can be parallel with 4-9)
    ↓
Phase 11: Testing  (write alongside each phase)
```

Phase 8 (Wait-for-Completion Mode) is **deferred** — schema exists, parser recognizes syntax, but UI logic not implemented.

---

## Success Criteria

1. ✅ Recurrence rules persisted to DB on task creation
2. ✅ Completing a recurring task spawns next instance
3. ✅ ♻ indicator visible next to recurring tasks
4. ✅ Recurring view (key 6) shows all recurring tasks
5. ✅ Edit preserves and updates recurrence pattern correctly
6. ✅ `every! day` sets `anchor_mode: Completion` and next spawn uses completion date
7. ✅ All unit and integration tests pass
8. ✅ No orphaned rules in DB after editing to remove recurrence
9. ✅ `wait!` prefix recognized and stored (even if UI logic is deferred)

---

## Files Modified Summary

| File | Type | Changes |
|------|------|---------|
| `migrations/0004_recurrence_enhancements.sql` | New | Schema migration |
| `crates/todomrs-core/src/domain.rs` | Edit | Add AnchorMode, update RecurrenceRule |
| `crates/todomrs-core/src/parser.rs` | Edit | Add wait!/every! parsing, ParsedTask fields |
| `crates/todomrs-store/src/recurrence_store.rs` | New | Full CRUD |
| `crates/todomrs-store/src/lib.rs` | Edit | Export new module |
| `crates/todomrs-tui/src/main.rs` | Edit | Inject store |
| `crates/todomrs-tui/src/app.rs` | Edit | Multiple changes (see phases) |
| `crates/todomrs-tui/src/ui.rs` | Edit | ♻ indicator, Recurring view, sidebar, help |
| `crates/todomrs-sync/src/operations.rs` | Edit | RecurrenceRuleCreate/Update variants |

---

## Notes for Implementer

1. **HashMap key = rule.id, not task_id.** This is the most common mistake. `refresh_tasks` loads rules keyed by `r.id`. `toggle_complete` looks up via `task.recurrence_rule_id`. `ui.rs` renders via `task.recurrence_rule_id`. All three must be consistent.
2. **Don't forget `use std::collections::HashMap;`** in app.rs.
3. **Both `wait!` and `every!` can appear together**: `wait! every! day` → wait_for_completion + completion-anchored.
4. **Edit path is the hardest part.** Phase 7.2 (save logic) is where most bugs will surface. Test thoroughly.
5. **Use serde_json for enum serialization**, not `format!("{:?}", ...)`.
6. **RecurrenceEngine is already tested** — just call it, don't re-test it.
7. **Soft-delete is safe for FK cascade** — no hard-deletes happen during normal use.
8. **Test each phase manually** before moving to the next.
