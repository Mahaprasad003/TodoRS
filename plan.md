# Phase 6.5 Implementation Plan: TUI Polish

## Executive Summary

Phase 6.5 addresses critical UX gaps in the TUI before backend/sync work begins. This phase fixes a weekday resolution bug, adds task editing, search functionality, completed view, input field navigation, project auto-creation, sidebar project display, and updates help text.

**Total estimated effort:** ~8 hours (1 work day)  
**Files to modify:** 4 files  
**Lines changed:** ~370 lines  
**Critical path:** Task 1 → Task 6 → Task 3

---

## Prerequisites Checklist

Before starting Phase 6.5, verify:
- [ ] Phase 6 is complete and committed
- [ ] `cargo test` passes with no failures
- [ ] `cargo run --bin todomrs` launches successfully
- [ ] All stores (TaskStore, OperationStore, ProjectStore) are working
- [ ] Parser and recurrence engine are functional
- [ ] Operation log is recording operations correctly

---

## Implementation Order

Execute tasks in this order to minimize dependencies and enable parallel work:

1. **Task 1** - Fix weekday resolution bug (foundation, no deps)
2. **Task 6** - Input field navigation (needed by Tasks 3, 4, 5)
3. **Tasks 3, 4, 5** - Edit task, Search, Completed view (can be done in parallel)
4. **Task 2** - Wire up +project auto-create (requires ProjectStore + parser changes)
5. **Task 7** - Sidebar project display (depends on Task 2)
6. **Task 8** - Update help text (last, after all features complete)

---

## Task 1: Fix Weekday Resolution Bug

### Problem Statement

The `next_weekday()` function in `crates/todomrs-core/src/parser.rs` (lines 280-287) has a bug where it never returns the same day as the input. The current implementation always adds at least 1 day due to the `+ 1` at the end of the calculation.

**Current buggy code (lines 280-287):**
```rust
pub fn next_weekday(from: NaiveDate, weekday: Weekday) -> NaiveDate {
    let from_weekday = from.weekday();
    let days_ahead = (weekday.num_days_from_monday() as i32
        - from_weekday.num_days_from_monday() as i32
        + 6)
        % 7
        + 1;
    from + Duration::days(days_ahead as i64)
}
```

### Required Behavior

- If today is Monday and user types `friday`, return this Friday (4 days ahead)
- If today is Saturday and user types `friday`, return next Friday (6 days ahead)
- If today is Friday and user types `friday`, return next Friday (7 days ahead)
- **General rule:** If target weekday hasn't passed yet this week, return it; if it has passed or is today, return next week's occurrence

### Changes

**File:** `crates/todomrs-core/src/parser.rs`

**Location:** Lines 280-287 (replace the entire `next_weekday` function)

**New implementation:**
```rust
pub fn next_weekday(from: NaiveDate, weekday: Weekday) -> NaiveDate {
    let from_weekday = from.weekday();
    let target_day = weekday.num_days_from_monday() as i32;
    let current_day = from_weekday.num_days_from_monday() as i32;
    
    // If same day, always return next week (7 days)
    if target_day == current_day {
        return from + Duration::days(7);
    }
    
    // Otherwise calculate days ahead (0-6)
    let days_ahead = (target_day - current_day + 7) % 7;
    from + Duration::days(days_ahead as i64)
}
```

**Location:** Lines 401-412 (verify existing tests still pass)

**Existing tests that should pass:**
- `test_next_weekday_same_day_is_next_week` (lines 401-406) - verifies Monday→Monday = 7 days
- `test_next_weekday_next_day` (lines 408-412) - verifies Thursday→Friday = 1 day
- `test_next_weekday_wrap_around` (lines 414-418) - verifies Thursday→Wednesday = 6 days

**New test to add after line 418:**
```rust
#[test]
fn test_next_weekday_same_day_returns_seven_days() {
    // Monday 2026-06-15
    let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
    assert_eq!(date.weekday(), Weekday::Mon);
    let next = ParsedTask::next_weekday(date, Weekday::Mon);
    // Should be 7 days later = Monday 2026-06-22
    assert_eq!(next, NaiveDate::from_ymd_opt(2026, 6, 22).unwrap());
}

#[test]
fn test_next_weekday_this_week() {
    // Monday 2026-06-15
    let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
    assert_eq!(date.weekday(), Weekday::Mon);
    
    // Friday should be this Friday (4 days)
    let next_fri = ParsedTask::next_weekday(date, Weekday::Fri);
    assert_eq!(next_fri, NaiveDate::from_ymd_opt(2026, 6, 19).unwrap());
    
    // Wednesday should be this Wednesday (2 days)
    let next_wed = ParsedTask::next_weekday(date, Weekday::Wed);
    assert_eq!(next_wed, NaiveDate::from_ymd_opt(2026, 6, 17).unwrap());
}
```

### Acceptance Criteria

- [ ] All existing tests pass: `cargo test --package todomrs-core`
- [ ] New test `test_next_weekday_same_day_returns_seven_days` passes
- [ ] New test `test_next_weekday_this_week` passes
- [ ] Manual verification: On Monday, typing `friday` returns this Friday (4 days ahead)
- [ ] Manual verification: On Friday, typing `friday` returns next Friday (7 days ahead)
- [ ] Manual verification: On Saturday, typing `friday` returns next Friday (6 days ahead)

### Risk Assessment

**Risk Level:** Low  
**Rationale:** Well-tested math fix with clear acceptance criteria. The existing tests already validate the correct behavior, and the new implementation is straightforward.

---

## Task 2: Wire Up +project Auto-Create

### Problem Statement

The parser extracts `+project` from input (line 68-73 in parser.rs), but the TUI doesn't use this information. When a user types `task +newproject`, the project should be auto-created if it doesn't exist, and the task should be assigned to it.

### Changes Required

#### Change 2.1: Add ProjectStore::find_by_name method

**File:** `crates/todomrs-store/src/project_store.rs`

**Location:** After line 61 (after `get_all` method)

**Add new method:**
```rust
pub async fn find_by_name(&self, user_id: Uuid, name: &str) -> Result<Option<Project>> {
    let row: Option<ProjectRow> = sqlx::query_as(
        "SELECT * FROM projects WHERE user_id = ? AND name = ? AND archived_at IS NULL"
    )
    .bind(user_id)
    .bind(name)
    .fetch_optional(&self.pool)
    .await?;
    Ok(row.map(ProjectRow::into_project))
}
```

#### Change 2.2: Add Project::new constructor

**File:** `crates/todomrs-core/src/domain.rs`

**Location:** After line 57 (after Project struct definition, before Tag struct)

**Add new impl block:**
```rust
impl Project {
    pub fn new(user_id: Uuid, name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            name,
            color: None,
            sort_order: 0,
            created_at: now,
            updated_at: now,
            archived_at: None,
        }
    }
}
```

#### Change 2.3: Modify parser return type to expose project name

**File:** `crates/todomrs-core/src/parser.rs`

**Location:** Line 145 (change function signature)

**Current:**
```rust
pub fn create_task_from_input(input: &str, user_id: Uuid) -> (Task, Option<RecurrenceRule>) {
```

**New:**
```rust
pub fn create_task_from_input(input: &str, user_id: Uuid) -> (Task, Option<RecurrenceRule>, Option<String>) {
```

**Location:** Line 196 (change return statement)

**Current:**
```rust
(task, recurrence_rule)
```

**New:**
```rust
(task, recurrence_rule, parsed.project)
```

#### Change 2.4: Update parser tests to match new return type

**File:** `crates/todomrs-core/src/parser.rs`

**Location:** Lines 421-450 (test_create_task_from_complex_input)

**Current (line 423):**
```rust
let (task, rule) = NaturalLanguageParser::create_task_from_input(input, user_id);
```

**New:**
```rust
let (task, rule, project) = NaturalLanguageParser::create_task_from_input(input, user_id);
```

**Add after line 440:**
```rust
assert_eq!(project, Some("vit".to_string()));
```

**Location:** Lines 443-450 (test_create_task_simple)

**Current (line 445):**
```rust
let (task, rule) = NaturalLanguageParser::create_task_from_input("Buy milk", user_id);
```

**New:**
```rust
let (task, rule, project) = NaturalLanguageParser::create_task_from_input("Buy milk", user_id);
```

**Add after line 449:**
```rust
assert_eq!(project, None);
```

**Location:** Lines 452-458 (test_create_task_with_every_2_days)

**Current (line 454):**
```rust
let (task, rule) =
    NaturalLanguageParser::create_task_from_input("Water plants every 2 days", user_id);
```

**New:**
```rust
let (task, rule, _project) =
    NaturalLanguageParser::create_task_from_input("Water plants every 2 days", user_id);
```

#### Change 2.5: Inject ProjectStore into App

**File:** `crates/todomrs-tui/src/main.rs`

**Location:** Line 12 (add import)

**Add:**
```rust
use todomrs_store::{Database, OperationStore, ProjectStore, TaskStore};
```

**Location:** Line 51 (after task_store and op_store initialization)

**Add:**
```rust
let project_store = ProjectStore::new(db.pool().clone());
```

**Location:** Line 69 (update App::new call)

**Current:**
```rust
let mut app = App::new(user_id, device_id, task_store, op_store);
```

**New:**
```rust
let mut app = App::new(user_id, device_id, task_store, op_store, project_store);
```

#### Change 2.6: Add project_store field to App struct

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** Line 5 (add import)

**Current:**
```rust
use todomrs_store::{OperationStore, TaskStore};
```

**New:**
```rust
use todomrs_store::{OperationStore, ProjectStore, TaskStore};
```

**Location:** Line 32 (add field to App struct)

**Add:**
```rust
pub project_store: ProjectStore,
```

**Location:** Line 64 (update App::new signature)

**Current:**
```rust
pub fn new(
    user_id: Uuid,
    device_id: Uuid,
    task_store: TaskStore,
    op_store: OperationStore,
) -> Self {
```

**New:**
```rust
pub fn new(
    user_id: Uuid,
    device_id: Uuid,
    task_store: TaskStore,
    op_store: OperationStore,
    project_store: ProjectStore,
) -> Self {
```

**Location:** Line 78 (add field initialization)

**Add:**
```rust
project_store,
```

**Location:** Line 48 (add field to Debug impl)

**Add:**
```rust
.field("project_store", &"ProjectStore")
```

#### Change 2.7: Wire up project auto-creation in create_task_from_input

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** Lines 186-204 (create_task_from_input method)

**Current:**
```rust
async fn create_task_from_input(&mut self) -> Result<()> {
    let input = self.input_buffer.trim().to_string();
    if input.is_empty() {
        return Ok(());
    }

    let (task, _recurrence_rule) =
        NaturalLanguageParser::create_task_from_input(&input, self.user_id);

    // Persist task (recurrence rule storage will be wired in a later phase)
    self.task_store.create(&task).await?;

    // Record operation for sync
    let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
    let op = Operation::create_task(self.user_id, self.device_id, seq, &task);
    self.op_store.append(&op).await?;

    self.status_message = Some(format!("Created: {}", task.title));
    self.refresh_tasks().await?;
    Ok(())
}
```

**New:**
```rust
async fn create_task_from_input(&mut self) -> Result<()> {
    let input = self.input_buffer.trim().to_string();
    if input.is_empty() {
        return Ok(());
    }

    let (mut task, _recurrence_rule, project_name) =
        NaturalLanguageParser::create_task_from_input(&input, self.user_id);

    // Handle project assignment
    if let Some(project_name) = project_name {
        let project = self.project_store.find_by_name(self.user_id, &project_name).await?;
        let project_id = match project {
            Some(p) => p.id,
            None => {
                let new_project = Project::new(self.user_id, project_name.clone());
                self.project_store.create(&new_project).await?;
                
                // Record project creation operation
                let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
                let op = Operation {
                    op_id: Uuid::new_v4(),
                    user_id: self.user_id,
                    device_id: self.device_id,
                    seq,
                    entity: todomrs_sync::operations::Entity::Project,
                    entity_id: new_project.id,
                    op_type: todomrs_sync::operations::OperationType::Create,
                    payload: todomrs_sync::operations::OperationPayload::ProjectCreate {
                        name: new_project.name.clone(),
                        color: new_project.color.clone(),
                        sort_order: new_project.sort_order,
                    },
                    created_at: chrono::Utc::now(),
                    synced_at: None,
                };
                self.op_store.append(&op).await?;
                
                new_project.id
            }
        };
        task.project_id = Some(project_id);
    }

    // Persist task (recurrence rule storage will be wired in a later phase)
    self.task_store.create(&task).await?;

    // Record operation for sync
    let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
    let op = Operation::create_task(self.user_id, self.device_id, seq, &task);
    self.op_store.append(&op).await?;

    self.status_message = Some(format!("Created: {}", task.title));
    self.refresh_tasks().await?;
    Ok(())
}
```

### Acceptance Criteria

- [ ] `cargo test --package todomrs-core` passes (parser tests updated)
- [ ] `cargo test --package todomrs-store` passes
- [ ] `cargo test --package todomrs-tui` passes
- [ ] `cargo build` succeeds with no errors
- [ ] Manual test: Create task with `Buy groceries +shopping`, verify project "shopping" is created in database
- [ ] Manual test: Create second task with `Buy milk +shopping`, verify same project is reused (not duplicated)
- [ ] Manual test: Verify task.project_id is set correctly for both tasks
- [ ] Manual test: Verify project creation operation is recorded in operations table

### Risk Assessment

**Risk Level:** High  
**Rationale:** 
- Requires changes to main.rs initialization flow
- Requires parser signature change that affects all callers
- Must ensure database migrations exist for projects table (already exist in migration 0001_init.sql)
- Must handle case where project already exists vs needs creation

**Mitigation:**
- Run full test suite after each change
- Verify database schema before testing
- Test with both new and existing projects

---

## Task 3: Edit Task (e key)

### Problem Statement

Users cannot edit task titles after creation. This is a critical UX gap for a daily-driver task manager.

### Changes Required

#### Change 3.1: Add EditingTask variant to InputMode enum

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** Lines 16-19 (InputMode enum)

**Current:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}
```

**New:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
    EditingTask(usize),  // stores task index being edited
}
```

#### Change 3.2: Add cursor_position field to App struct

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** Line 35 (add field to App struct)

**Add:**
```rust
pub cursor_position: usize,
```

**Location:** Line 79 (initialize in App::new)

**Add:**
```rust
cursor_position: 0,
```

**Location:** Line 51 (add to Debug impl)

**Add:**
```rust
.field("cursor_position", &self.cursor_position)
```

#### Change 3.3: Bind 'e' key in Normal mode

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** After line 161 (after 'd' key binding in Normal mode match)

**Add:**
```rust
KeyCode::Char('e') if key.modifiers.is_empty() => {
    let filtered = self.filtered_tasks();
    if self.selected_index < filtered.len() {
        let task = filtered[self.selected_index];
        self.input_buffer = task.title.clone();
        self.cursor_position = self.input_buffer.len();
        self.input_mode = InputMode::EditingTask(self.selected_index);
    }
}
```

#### Change 3.4: Handle EditingTask mode in event handler

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** After line 183 (after Editing mode match arm)

**Add new match arm:**
```rust
InputMode::EditingTask(task_idx) => match key.code {
    KeyCode::Esc => {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        self.cursor_position = 0;
    }
    KeyCode::Enter => {
        self.update_task_title(task_idx).await?;
        self.input_buffer.clear();
        self.cursor_position = 0;
        self.input_mode = InputMode::Normal;
    }
    KeyCode::Char(c) => {
        self.input_buffer.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }
    KeyCode::Backspace => {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input_buffer.remove(self.cursor_position);
        }
    }
    KeyCode::Left => {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }
    KeyCode::Right => {
        if self.cursor_position < self.input_buffer.len() {
            self.cursor_position += 1;
        }
    }
    KeyCode::Home => {
        self.cursor_position = 0;
    }
    KeyCode::End => {
        self.cursor_position = self.input_buffer.len();
    }
    _ => {}
},
```

#### Change 3.5: Add update_task_title method

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** After line 287 (after delete_task method)

**Add new method:**
```rust
async fn update_task_title(&mut self, task_idx: usize) -> Result<()> {
    let filtered = self.filtered_tasks();
    if task_idx >= filtered.len() {
        return Ok(());
    }

    let mut task = filtered[task_idx].clone();
    let new_title = self.input_buffer.trim().to_string();
    
    if new_title.is_empty() || new_title == task.title {
        return Ok(());
    }

    let old_title = task.title.clone();
    task.title = new_title.clone();
    task.updated_at = chrono::Utc::now();

    self.task_store.update(&task).await?;

    // Record update operation for sync (use existing helper)
    let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
    let op = Operation::update_task_title(self.user_id, self.device_id, seq, task.id, new_title.clone());
    self.op_store.append(&op).await?;

    self.status_message = Some(format!("Updated: {} → {}", old_title, new_title));
    self.refresh_tasks().await?;
    Ok(())
}
```

#### Change 3.6: Update UI to show different prompt for EditingTask mode

**File:** `crates/todomrs-tui/src/ui.rs`

**Location:** Lines 160-163 (draw_input_field title match)

**Current:**
```rust
let title = match app.input_mode {
    InputMode::Normal => "Press 'a' to add task",
    InputMode::Editing => "Add task (Enter to save, Esc to cancel)",
};
```

**New:**
```rust
let title = match app.input_mode {
    InputMode::Normal => "Press 'a' to add task",
    InputMode::Editing => "Add task (Enter to save, Esc to cancel)",
    InputMode::EditingTask(_) => "Edit task (Enter to save, Esc to cancel)",
};
```

**Location:** Lines 175-178 (cursor rendering)

**Current:**
```rust
if app.input_mode == InputMode::Editing {
    let cursor_x = area.x + (app.input_buffer.len() as u16).min(area.width.saturating_sub(2)) + 1;
    f.set_cursor(cursor_x, area.y + 1);
}
```

**New:**
```rust
if matches!(app.input_mode, InputMode::Editing | InputMode::EditingTask(_)) {
    let cursor_pos = app.cursor_position.min(app.input_buffer.len());
    let cursor_x = area.x + (cursor_pos as u16).min(area.width.saturating_sub(2)) + 1;
    f.set_cursor(cursor_x, area.y + 1);
}
```

### Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] Manual test: Select task, press 'e', verify input field shows current task title
- [ ] Manual test: Edit title, press Enter, verify task is updated in database
- [ ] Manual test: Verify operation is recorded in operations table
- [ ] Manual test: Press Esc during edit, verify no changes made
- [ ] Manual test: Arrow keys move cursor left/right in input field
- [ ] Manual test: Home/End keys move cursor to start/end
- [ ] Manual test: Status message shows "Updated: old_title → new_title"

### Risk Assessment

**Risk Level:** Medium  
**Rationale:** 
- Requires careful handling of cursor position
- Must ensure task index remains valid after filtering
- Operation recording uses existing helper (low risk)

---

## Task 4: Search Functionality (/ key)

### Problem Statement

Users cannot search for tasks by title. This is essential for navigating large task lists.

### Changes Required

#### Change 4.1: Add Searching variant to InputMode enum

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** Lines 16-20 (InputMode enum, after Task 3 changes)

**Current:**
```rust
pub enum InputMode {
    Normal,
    Editing,
    EditingTask(usize),
}
```

**New:**
```rust
pub enum InputMode {
    Normal,
    Editing,
    EditingTask(usize),
    Searching,
}
```

#### Change 4.2: Add search fields to App struct

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** After line 35 (add fields to App struct)

**Add:**
```rust
pub search_query: String,
```

**Location:** After line 79 (initialize in App::new)

**Add:**
```rust
search_query: String::new(),
```

**Location:** In Debug impl (add field)

**Add:**
```rust
.field("search_query", &self.search_query)
```

#### Change 4.3: Bind '/' key in Normal mode

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** After the 'e' key binding (after Task 3 changes)

**Add:**
```rust
KeyCode::Char('/') if key.modifiers.is_empty() => {
    self.input_mode = InputMode::Searching;
    self.search_query.clear();
    self.cursor_position = 0;
}
```

#### Change 4.4: Handle Searching mode in event handler

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** After the EditingTask match arm (after Task 3 changes)

**Add new match arm:**
```rust
InputMode::Searching => match key.code {
    KeyCode::Esc => {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
        self.cursor_position = 0;
    }
    KeyCode::Enter => {
        self.input_mode = InputMode::Normal;
        self.cursor_position = 0;
        // Keep search_query active for filtering
    }
    KeyCode::Char(c) => {
        self.search_query.push(c);
        self.cursor_position = self.search_query.len();
    }
    KeyCode::Backspace => {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.search_query.remove(self.cursor_position);
        }
    }
    KeyCode::Left => {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }
    KeyCode::Right => {
        if self.cursor_position < self.search_query.len() {
            self.cursor_position += 1;
        }
    }
    KeyCode::Home => {
        self.cursor_position = 0;
    }
    KeyCode::End => {
        self.cursor_position = self.search_query.len();
    }
    _ => {}
},
```

#### Change 4.5: Update filtered_tasks to apply search filter

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** Lines 94-107 (filtered_tasks method)

**Current:**
```rust
pub fn filtered_tasks(&self) -> Vec<&Task> {
    let today = chrono::Utc::now().date_naive();
    match self.current_view {
        View::Inbox => self.tasks.iter().collect(),
        View::Today => self
            .tasks
            .iter()
            .filter(|t| t.due_at.map(|dt| dt.date_naive() == today).unwrap_or(false))
            .collect(),
        View::Upcoming => self
            .tasks
            .iter()
            .filter(|t| t.due_at.map(|dt| dt.date_naive() > today).unwrap_or(false))
            .collect(),
        View::Projects => Vec::new(),
    }
}
```

**New:**
```rust
pub fn filtered_tasks(&self) -> Vec<&Task> {
    let today = chrono::Utc::now().date_naive();
    let mut tasks: Vec<&Task> = match self.current_view {
        View::Inbox => self.tasks.iter().collect(),
        View::Today => self
            .tasks
            .iter()
            .filter(|t| t.due_at.map(|dt| dt.date_naive() == today).unwrap_or(false))
            .collect(),
        View::Upcoming => self
            .tasks
            .iter()
            .filter(|t| t.due_at.map(|dt| dt.date_naive() > today).unwrap_or(false))
            .collect(),
        View::Projects => Vec::new(),
        View::Completed => self.tasks.iter()
            .filter(|t| t.status == TaskStatus::Completed && t.deleted_at.is_none())
            .collect(),
    };

    // Apply search filter if active
    if !self.search_query.is_empty() {
        let query = self.search_query.to_lowercase();
        tasks.retain(|t| t.title.to_lowercase().contains(&query));
    }

    tasks
}
```

#### Change 4.6: Update UI to show search query

**File:** `crates/todomrs-tui/src/ui.rs`

**Location:** Lines 160-164 (draw_input_field title match, after Task 3 changes)

**Current:**
```rust
let title = match app.input_mode {
    InputMode::Normal => "Press 'a' to add task",
    InputMode::Editing => "Add task (Enter to save, Esc to cancel)",
    InputMode::EditingTask(_) => "Edit task (Enter to save, Esc to cancel)",
};
```

**New:**
```rust
let title = match app.input_mode {
    InputMode::Normal => "Press 'a' to add task",
    InputMode::Editing => "Add task (Enter to save, Esc to cancel)",
    InputMode::EditingTask(_) => "Edit task (Enter to save, Esc to cancel)",
    InputMode::Searching => {
        if app.search_query.is_empty() {
            "Search (type query, Enter to confirm, Esc to cancel)"
        } else {
            "Search active (Esc to clear)"
        }
    }
};
```

**Location:** Lines 165-173 (input paragraph rendering)

**Current:**
```rust
let input = Paragraph::new(app.input_buffer.as_str())
    .style(match app.input_mode {
        InputMode::Normal => Style::default().fg(Color::DarkGray),
        InputMode::Editing => Style::default().fg(Color::White),
    })
    .block(Block::default().borders(Borders::ALL).title(title));
```

**New:**
```rust
let display_text = match app.input_mode {
    InputMode::Searching => app.search_query.as_str(),
    _ => app.input_buffer.as_str(),
};

let input = Paragraph::new(display_text)
    .style(match app.input_mode {
        InputMode::Normal => Style::default().fg(Color::DarkGray),
        _ => Style::default().fg(Color::White),
    })
    .block(Block::default().borders(Borders::ALL).title(title));
```

**Location:** Lines 175-179 (cursor rendering, after Task 3 changes)

**Current:**
```rust
if matches!(app.input_mode, InputMode::Editing | InputMode::EditingTask(_)) {
    let cursor_pos = app.cursor_position.min(app.input_buffer.len());
    let cursor_x = area.x + (cursor_pos as u16).min(area.width.saturating_sub(2)) + 1;
    f.set_cursor(cursor_x, area.y + 1);
}
```

**New:**
```rust
if matches!(app.input_mode, InputMode::Editing | InputMode::EditingTask(_) | InputMode::Searching) {
    let buffer_len = match app.input_mode {
        InputMode::Searching => app.search_query.len(),
        _ => app.input_buffer.len(),
    };
    let cursor_pos = app.cursor_position.min(buffer_len);
    let cursor_x = area.x + (cursor_pos as u16).min(area.width.saturating_sub(2)) + 1;
    f.set_cursor(cursor_x, area.y + 1);
}
```

**Location:** In draw_main_content, add search indicator (after line 65)

**Add:**
```rust
let title = if !app.search_query.is_empty() {
    format!("{} [search: {}]", title, app.search_query)
} else {
    title.to_string()
};
```

### Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] Manual test: Press '/', verify input field shows search prompt
- [ ] Manual test: Type query, verify tasks are filtered by title (case-insensitive)
- [ ] Manual test: Press Enter, verify search stays active and results remain filtered
- [ ] Manual test: Press Esc, verify search is cleared and all tasks shown
- [ ] Manual test: Arrow keys move cursor in search input
- [ ] Manual test: Search indicator shown in main content area when search is active

### Risk Assessment

**Risk Level:** Medium  
**Rationale:** 
- Search state must be properly managed (clearing, restoring)
- Case-insensitive matching requires to_lowercase() on every comparison (performance consideration for large task lists)
- Interaction with other input modes must be carefully tested

---

## Task 5: Completed/Archive View

### Problem Statement

Users cannot view or clear completed tasks. This is essential for task lifecycle management.

### Changes Required

#### Change 5.1: Add Completed variant to View enum

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** Lines 9-14 (View enum)

**Current:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Inbox,
    Today,
    Upcoming,
    Projects,
}
```

**New:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Inbox,
    Today,
    Upcoming,
    Projects,
    Completed,
}
```

#### Change 5.2: Update filtered_tasks for Completed view

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** Lines 94-107 (filtered_tasks method, already updated in Task 4)

**Verify Completed view is included:**
```rust
View::Completed => self.tasks.iter()
    .filter(|t| t.status == TaskStatus::Completed && t.deleted_at.is_none())
    .collect(),
```

#### Change 5.3: Bind '5' key for Completed view

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** After the '4' key binding (around line 165)

**Add:**
```rust
KeyCode::Char('5') => {
    self.current_view = View::Completed;
    self.selected_index = 0;
}
```

#### Change 5.4: Bind 'C' key to clear all completed

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** After the '5' key binding

**Add:**
```rust
KeyCode::Char('C') if key.modifiers.is_empty() => {
    self.clear_completed().await?
}
```

#### Change 5.5: Add clear_completed method

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** After the update_task_title method (after Task 3 changes)

**Add new method:**
```rust
async fn clear_completed(&mut self) -> Result<()> {
    let completed: Vec<Task> = self.tasks.iter()
        .filter(|t| t.status == TaskStatus::Completed && t.deleted_at.is_none())
        .cloned()
        .collect();

    let count = completed.len();
    if count == 0 {
        self.status_message = Some("No completed tasks to clear".to_string());
        return Ok(());
    }

    for task in completed {
        self.task_store.soft_delete(task.id).await?;
        
        // Record delete operation
        let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
        let op = Operation {
            op_id: Uuid::new_v4(),
            user_id: self.user_id,
            device_id: self.device_id,
            seq,
            entity: todomrs_sync::operations::Entity::Task,
            entity_id: task.id,
            op_type: todomrs_sync::operations::OperationType::Delete,
            payload: todomrs_sync::operations::OperationPayload::Delete,
            created_at: chrono::Utc::now(),
            synced_at: None,
        };
        self.op_store.append(&op).await?;
    }

    self.status_message = Some(format!("Cleared {} completed tasks", count));
    self.selected_index = 0;
    self.refresh_tasks().await?;
    Ok(())
}
```

#### Change 5.6: Update UI sidebar to include Completed

**File:** `crates/todomrs-tui/src/ui.rs`

**Location:** Lines 38-60 (draw_sidebar function)

**Current:**
```rust
fn draw_sidebar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items = vec![
        ListItem::new("Inbox"),
        ListItem::new("Today"),
        ListItem::new("Upcoming"),
        ListItem::new("Projects"),
    ];

    let selected = match app.current_view {
        View::Inbox => 0,
        View::Today => 1,
        View::Upcoming => 2,
        View::Projects => 3,
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Views"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ratatui::widgets::ListState::default().with_selected(Some(selected));
    f.render_stateful_widget(list, area, &mut state);
}
```

**New:**
```rust
fn draw_sidebar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items = vec![
        ListItem::new("Inbox"),
        ListItem::new("Today"),
        ListItem::new("Upcoming"),
        ListItem::new("Projects"),
        ListItem::new("Completed"),
    ];

    let selected = match app.current_view {
        View::Inbox => 0,
        View::Today => 1,
        View::Upcoming => 2,
        View::Projects => 3,
        View::Completed => 4,
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Views"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ratatui::widgets::ListState::default().with_selected(Some(selected));
    f.render_stateful_widget(list, area, &mut state);
}
```

#### Change 5.7: Update main content title for Completed view

**File:** `crates/todomrs-tui/src/ui.rs`

**Location:** Lines 62-70 (draw_main_content title match)

**Current:**
```rust
let title = match app.current_view {
    View::Inbox => "Inbox",
    View::Today => "Today",
    View::Upcoming => "Upcoming",
    View::Projects => "Projects",
};
```

**New:**
```rust
let title = match app.current_view {
    View::Inbox => "Inbox",
    View::Today => "Today",
    View::Upcoming => "Upcoming",
    View::Projects => "Projects",
    View::Completed => "Completed",
};
```

#### Change 5.8: Update status bar to include 'C' shortcut

**File:** `crates/todomrs-tui/src/ui.rs`

**Location:** Lines 195-222 (draw_status_bar function)

**Current:**
```rust
let status = Line::from(vec![
    Span::styled(
        " TodoRS ",
        Style::default().bg(Color::Blue).fg(Color::White),
    ),
    Span::raw(format!(" {} ", view_name)),
    Span::raw("│ "),
    Span::styled("q", Style::default().fg(Color::Yellow)),
    Span::raw(" Quit "),
    Span::styled("?", Style::default().fg(Color::Yellow)),
    Span::raw(" Help "),
    Span::styled("x", Style::default().fg(Color::Yellow)),
    Span::raw(" Toggle "),
    Span::styled("d", Style::default().fg(Color::Yellow)),
    Span::raw(" Del "),
]);
```

**New:**
```rust
let status = Line::from(vec![
    Span::styled(
        " TodoRS ",
        Style::default().bg(Color::Blue).fg(Color::White),
    ),
    Span::raw(format!(" {} ", view_name)),
    Span::raw("│ "),
    Span::styled("q", Style::default().fg(Color::Yellow)),
    Span::raw(" Quit "),
    Span::styled("?", Style::default().fg(Color::Yellow)),
    Span::raw(" Help "),
    Span::styled("a", Style::default().fg(Color::Yellow)),
    Span::raw(" Add "),
    Span::styled("e", Style::default().fg(Color::Yellow)),
    Span::raw(" Edit "),
    Span::styled("/", Style::default().fg(Color::Yellow)),
    Span::raw(" Search "),
    Span::styled("x", Style::default().fg(Color::Yellow)),
    Span::raw(" Toggle "),
    Span::styled("d", Style::default().fg(Color::Yellow)),
    Span::raw(" Del "),
    Span::styled("C", Style::default().fg(Color::Yellow)),
    Span::raw(" Clear "),
]);
```

#### Change 5.9: Update view_name match for Completed

**File:** `crates/todomrs-tui/src/ui.rs`

**Location:** Lines 195-202 (draw_status_bar view_name match)

**Current:**
```rust
let view_name = match app.current_view {
    View::Inbox => "Inbox",
    View::Today => "Today",
    View::Upcoming => "Upcoming",
    View::Projects => "Projects",
};
```

**New:**
```rust
let view_name = match app.current_view {
    View::Inbox => "Inbox",
    View::Today => "Today",
    View::Upcoming => "Upcoming",
    View::Projects => "Projects",
    View::Completed => "Completed",
};
```

### Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] Manual test: Press '5', verify Completed view is shown
- [ ] Manual test: Verify only completed, non-deleted tasks are shown
- [ ] Manual test: Press 'C', verify all completed tasks are soft-deleted
- [ ] Manual test: Verify operations are recorded for each deleted task
- [ ] Manual test: Verify status message shows "Cleared X completed tasks"
- [ ] Manual test: Verify Completed view appears in sidebar
- [ ] Manual test: Verify status bar shows all shortcuts including 'C'

### Risk Assessment

**Risk Level:** Low  
**Rationale:** 
- Simple view addition following existing pattern
- Clear completed uses existing soft_delete method
- Operation recording follows existing pattern from delete_task

---

## Task 6: Input Field Navigation

### Problem Statement

Input fields only support appending characters. Users cannot navigate or edit within the input buffer, which is essential for efficient task editing.

### Changes Required

#### Change 6.1: Update Char/Backspace handling to use cursor position

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** In Editing mode match arm (lines 175-183)

**Current:**
```rust
InputMode::Editing => match key.code {
    KeyCode::Esc => {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
    }
    KeyCode::Enter => {
        self.create_task_from_input().await?;
        self.input_buffer.clear();
        self.input_mode = InputMode::Normal;
    }
    KeyCode::Char(c) => {
        self.input_buffer.push(c);
    }
    KeyCode::Backspace => {
        self.input_buffer.pop();
    }
    _ => {}
},
```

**New:**
```rust
InputMode::Editing => match key.code {
    KeyCode::Esc => {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        self.cursor_position = 0;
    }
    KeyCode::Enter => {
        self.create_task_from_input().await?;
        self.input_buffer.clear();
        self.cursor_position = 0;
        self.input_mode = InputMode::Normal;
    }
    KeyCode::Char(c) => {
        self.input_buffer.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }
    KeyCode::Backspace => {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input_buffer.remove(self.cursor_position);
        }
    }
    KeyCode::Left => {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }
    KeyCode::Right => {
        if self.cursor_position < self.input_buffer.len() {
            self.cursor_position += 1;
        }
    }
    KeyCode::Home => {
        self.cursor_position = 0;
    }
    KeyCode::End => {
        self.cursor_position = self.input_buffer.len();
    }
    _ => {}
},
```

**Location:** In 'a' key binding (line 145)

**Current:**
```rust
KeyCode::Char('a') if key.modifiers.is_empty() => {
    self.input_mode = InputMode::Editing;
    self.input_buffer.clear();
}
```

**New:**
```rust
KeyCode::Char('a') if key.modifiers.is_empty() => {
    self.input_mode = InputMode::Editing;
    self.input_buffer.clear();
    self.cursor_position = 0;
}
```

#### Change 6.2: Update cursor rendering for all editing modes

**File:** `crates/todomrs-tui/src/ui.rs`

**Location:** Lines 175-179 (cursor rendering, already updated in Tasks 3 and 4)

**Verify it handles all three editing modes:**
```rust
if matches!(app.input_mode, InputMode::Editing | InputMode::EditingTask(_) | InputMode::Searching) {
    let buffer_len = match app.input_mode {
        InputMode::Searching => app.search_query.len(),
        _ => app.input_buffer.len(),
    };
    let cursor_pos = app.cursor_position.min(buffer_len);
    let cursor_x = area.x + (cursor_pos as u16).min(area.width.saturating_sub(2)) + 1;
    f.set_cursor(cursor_x, area.y + 1);
}
```

### Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] Manual test: In add task mode, type text, use Left/Right arrows to navigate
- [ ] Manual test: Press Home, verify cursor moves to start
- [ ] Manual test: Press End, verify cursor moves to end
- [ ] Manual test: Use Backspace in middle of text, verify correct character deleted
- [ ] Manual test: Type in middle of text, verify characters inserted at cursor position
- [ ] Manual test: Cursor renders at correct position in all editing modes

### Risk Assessment

**Risk Level:** Low  
**Rationale:** 
- Cursor position tracking is straightforward
- Already integrated into Tasks 3, 4, 5
- No complex state management

---

## Task 7: Sidebar Project Display

### Problem Statement

Sidebar only shows views, not projects. Users need to see their projects with task counts for better organization.

### Changes Required

#### Change 7.1: Add project_counts field to App struct

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** Line 5 (add import)

**Current:**
```rust
use todomrs_store::{OperationStore, ProjectStore, TaskStore};
```

**New:**
```rust
use std::collections::HashMap;
use todomrs_store::{OperationStore, ProjectStore, TaskStore};
```

**Location:** After line 35 (add field to App struct)

**Add:**
```rust
pub project_counts: HashMap<Uuid, (String, usize)>,
```

**Location:** After line 79 (initialize in App::new)

**Add:**
```rust
project_counts: HashMap::new(),
```

**Location:** In Debug impl (add field)

**Add:**
```rust
.field("project_counts", &self.project_counts)
```

#### Change 7.2: Add refresh_project_counts method

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** After refresh_tasks method (after line 91)

**Add new method:**
```rust
pub async fn refresh_project_counts(&mut self) -> Result<()> {
    let projects = self.project_store.get_all(self.user_id).await?;
    let mut counts = HashMap::new();
    
    for project in projects {
        let count = self.tasks.iter()
            .filter(|t| t.project_id == Some(project.id))
            .count();
        counts.insert(project.id, (project.name.clone(), count));
    }
    
    self.project_counts = counts;
    Ok(())
}
```

#### Change 7.3: Call refresh_project_counts in refresh_tasks

**File:** `crates/todomrs-tui/src/app.rs`

**Location:** Lines 84-91 (refresh_tasks method)

**Current:**
```rust
pub async fn refresh_tasks(&mut self) -> Result<()> {
    self.tasks = self.task_store.get_all(self.user_id).await?;
    // Clamp selection to valid range
    let count = self.filtered_tasks().len();
    if count > 0 && self.selected_index >= count {
        self.selected_index = count.saturating_sub(1);
    }
    Ok(())
}
```

**New:**
```rust
pub async fn refresh_tasks(&mut self) -> Result<()> {
    self.tasks = self.task_store.get_all(self.user_id).await?;
    self.refresh_project_counts().await?;
    // Clamp selection to valid range
    let count = self.filtered_tasks().len();
    if count > 0 && self.selected_index >= count {
        self.selected_index = count.saturating_sub(1);
    }
    Ok(())
}
```

#### Change 7.4: Update sidebar to show projects with counts

**File:** `crates/todomrs-tui/src/ui.rs`

**Location:** Lines 38-60 (draw_sidebar function, after Task 5 changes)

**Current:**
```rust
fn draw_sidebar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items = vec![
        ListItem::new("Inbox"),
        ListItem::new("Today"),
        ListItem::new("Upcoming"),
        ListItem::new("Projects"),
        ListItem::new("Completed"),
    ];

    let selected = match app.current_view {
        View::Inbox => 0,
        View::Today => 1,
        View::Upcoming => 2,
        View::Projects => 3,
        View::Completed => 4,
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Views"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ratatui::widgets::ListState::default().with_selected(Some(selected));
    f.render_stateful_widget(list, area, &mut state);
}
```

**New:**
```rust
fn draw_sidebar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(2 + app.project_counts.len() as u16),
        ])
        .split(area);

    // Views section
    let items = vec![
        ListItem::new("Inbox"),
        ListItem::new("Today"),
        ListItem::new("Upcoming"),
        ListItem::new("Projects"),
        ListItem::new("Completed"),
    ];

    let selected = match app.current_view {
        View::Inbox => 0,
        View::Today => 1,
        View::Upcoming => 2,
        View::Projects => 3,
        View::Completed => 4,
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Views"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ratatui::widgets::ListState::default().with_selected(Some(selected));
    f.render_stateful_widget(list, chunks[0], &mut state);

    // Projects section
    let project_items: Vec<ListItem> = app.project_counts.values()
        .map(|(name, count)| ListItem::new(format!("{} ({})", name, count)))
        .collect();

    let project_list = List::new(project_items)
        .block(Block::default().borders(Borders::ALL).title("Projects"));
    f.render_widget(project_list, chunks[1]);
}
```

### Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] Manual test: Create tasks with projects, verify projects appear in sidebar
- [ ] Manual test: Verify task counts are correct for each project
- [ ] Manual test: Add/remove tasks, verify counts update
- [ ] Manual test: Verify projects without tasks show (0) count
- [ ] Manual test: Verify sidebar layout is correct with both Views and Projects sections

### Risk Assessment

**Risk Level:** Low  
**Rationale:** 
- Uses existing ProjectStore::get_all method
- Simple HashMap for counts
- Layout split is straightforward

---

## Task 8: Update Help Text

### Problem Statement

Help overlay doesn't document new shortcuts added in Phase 6.5.

### Changes Required

#### Change 8.1: Update help overlay with all new shortcuts

**File:** `crates/todomrs-tui/src/ui.rs`

**Location:** Lines 224-252 (draw_help function)

**Current:**
```rust
fn draw_help(f: &mut Frame) {
    let area = f.size();
    let help_h = (area.height / 2).min(area.height.saturating_sub(2)).max(14);
    let help_w = (area.width / 2).min(area.width.saturating_sub(4)).max(30);
    let help_area = ratatui::layout::Rect {
        x: (area.width.saturating_sub(help_w)) / 2,
        y: (area.height.saturating_sub(help_h)) / 2,
        width: help_w,
        height: help_h,
    };

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("q  — Quit"),
        Line::from("?  — Toggle help"),
        Line::from("a  — Add task"),
        Line::from("x  — Toggle complete"),
        Line::from("d  — Delete task"),
        Line::from("j/↓ — Next item"),
        Line::from("k/↑ — Previous item"),
        Line::from("1  — Inbox view"),
        Line::from("2  — Today view"),
        Line::from("3  — Upcoming view"),
        Line::from("4  — Projects view"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"));

    f.render_widget(Clear, help_area);
    f.render_widget(paragraph, help_area);
}
```

**New:**
```rust
fn draw_help(f: &mut Frame) {
    let area = f.size();
    let help_h = (area.height / 2).min(area.height.saturating_sub(2)).max(20);
    let help_w = (area.width / 2).min(area.width.saturating_sub(4)).max(40);
    let help_area = ratatui::layout::Rect {
        x: (area.width.saturating_sub(help_w)) / 2,
        y: (area.height.saturating_sub(help_h)) / 2,
        width: help_w,
        height: help_h,
    };

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  j/↓    — Next item"),
        Line::from("  k/↑    — Previous item"),
        Line::from("  1      — Inbox view"),
        Line::from("  2      — Today view"),
        Line::from("  3      — Upcoming view"),
        Line::from("  4      — Projects view"),
        Line::from("  5      — Completed view"),
        Line::from(""),
        Line::from("Task Operations:"),
        Line::from("  a      — Add task"),
        Line::from("  e      — Edit task"),
        Line::from("  x      — Toggle complete"),
        Line::from("  d      — Delete task"),
        Line::from("  /      — Search"),
        Line::from("  C      — Clear all completed"),
        Line::from("  ?      — Toggle help"),
        Line::from("  q      — Quit"),
        Line::from(""),
        Line::from("Input Mode:"),
        Line::from("  ←/→    — Move cursor"),
        Line::from("  Home   — Start of line"),
        Line::from("  End    — End of line"),
        Line::from("  Enter  — Confirm"),
        Line::from("  Esc    — Cancel"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"));

    f.render_widget(Clear, help_area);
    f.render_widget(paragraph, help_area);
}
```

### Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] Manual test: Press '?', verify help overlay shows all shortcuts
- [ ] Manual test: Verify help is organized by category (Navigation, Task Operations, Input Mode)
- [ ] Manual test: Verify all new shortcuts are documented (e, /, C, 5, arrow keys, Home, End)
- [ ] Manual test: Verify help overlay is large enough to display all text without truncation

### Risk Assessment

**Risk Level:** Low  
**Rationale:** 
- Pure UI change, no logic changes
- Help text is static
- Only requires updating the help_text vector

---

## Testing Strategy

### Unit Tests

**Parser tests (todomrs-core):**
- Test weekday resolution with all day combinations
- Test project extraction from input
- Test new return type for create_task_from_input

**Store tests (todomrs-store):**
- Test ProjectStore::find_by_name with existing project
- Test ProjectStore::find_by_name with non-existent project

**App tests (todomrs-tui):**
- Test edit task flow (if unit tests are added)
- Test search filtering logic
- Test completed view filtering

### Integration Tests

**End-to-end scenarios:**
1. Create task with +project, verify project created and task assigned
2. Create second task with same +project, verify project reused
3. Edit task title, verify database updated and operation recorded
4. Search for task, verify correct filtering
5. Clear completed tasks, verify all soft-deleted and operations recorded

### Manual Testing Checklist

- [ ] Monday: type `friday` → returns this Friday (4 days)
- [ ] Saturday: type `friday` → returns next Friday (6 days)
- [ ] Friday: type `friday` → returns next Friday (7 days)
- [ ] Create `task +newproject` → verify project exists in database
- [ ] Create `task2 +newproject` → verify same project reused
- [ ] Select task, press 'e', edit title, Enter → verify updated
- [ ] Press '/', type query → verify filtered results
- [ ] Press Esc → verify search cleared
- [ ] Press '5' → verify completed view shown
- [ ] Press 'C' → verify all completed tasks cleared
- [ ] Arrow keys in input → verify cursor moves
- [ ] Home/End keys → verify cursor jumps to start/end
- [ ] Sidebar shows projects with counts
- [ ] Add/remove tasks → verify counts update
- [ ] Press '?' → verify help shows all shortcuts

---

## Dependencies

### Task Dependencies

```
Task 1 (Weekday bug)
  ↓
Task 6 (Input navigation)
  ↓
Task 3 (Edit task) ─┐
Task 4 (Search) ────┤──→ Task 8 (Help text)
Task 5 (Completed) ─┘
  ↓
Task 2 (+project)
  ↓
Task 7 (Sidebar projects) ─→ Task 8
```

### Code Dependencies

- **Task 2** depends on:
  - ProjectStore::find_by_name (new method)
  - Project::new constructor (new method)
  - Parser signature change (breaking change)

- **Task 3** depends on:
  - Task 6 (cursor position tracking)
  - Operation::update_task_title (existing helper)

- **Task 4** depends on:
  - Task 6 (cursor position tracking)

- **Task 7** depends on:
  - Task 2 (ProjectStore injection)

- **Task 8** depends on:
  - All other tasks (documents all new shortcuts)

---

## Risks and Mitigations

### High Risk

1. **ProjectStore injection (Task 2)**
   - **Risk:** Requires changes to main.rs initialization flow
   - **Mitigation:** Test after each change, verify database migrations exist
   - **Fallback:** If injection fails, can defer to Phase 7

2. **Parser signature change (Task 2)**
   - **Risk:** Breaking change affects all callers
   - **Mitigation:** Update all tests immediately, run full test suite after change
   - **Fallback:** Can use separate method instead of changing signature

3. **Operation recording (Tasks 2, 3, 5)**
   - **Risk:** Easy to miss recording an operation
   - **Mitigation:** Use existing helper constructors where possible, verify operations table after each test
   - **Fallback:** Operations can be added in post-processing

### Medium Risk

1. **Cursor position tracking (Task 6)**
   - **Risk:** Edge cases with empty buffer, cursor at boundaries
   - **Mitigation:** Add bounds checking (cursor_position.min(buffer_len))
   - **Fallback:** Can simplify to append-only if too complex

2. **Search state management (Task 4)**
   - **Risk:** Need to properly restore view after search cancelled
   - **Mitigation:** Keep search_query separate from input_buffer, clear on Esc
   - **Fallback:** Can make search view-specific instead of global

### Low Risk

1. **Weekday calculation (Task 1)**
   - **Risk:** Math error in weekday calculation
   - **Mitigation:** Comprehensive tests with all day combinations
   - **Fallback:** Can use external crate (chrono-english) if needed

2. **UI updates (Tasks 3, 4, 5, 7, 8)**
   - **Risk:** Ratatui rendering issues
   - **Mitigation:** Test with different terminal sizes
   - **Fallback:** Can simplify UI if needed

---

## Success Criteria

Phase 6.5 is complete when:

1. ✅ Weekday resolution matches Todoist/Things behavior (same-day = 7 days)
2. ✅ +project auto-creates and reuses projects correctly
3. ✅ Can edit task titles with 'e' key
4. ✅ Can search tasks with '/' key (case-insensitive)
5. ✅ Can view and clear completed tasks with '5' and 'C' keys
6. ✅ Input field has full navigation (arrows, Home/End)
7. ✅ Sidebar shows project names with task counts
8. ✅ Help text documents all shortcuts organized by category
9. ✅ All operations recorded for future sync
10. ✅ No regressions in existing functionality (all tests pass)

---

## Post-Phase 6.5 Priorities

With TUI now feature-complete for daily use, the next phases should focus on:

**Phase 7: Backend API**
- REST API for operations
- Authentication (Supabase Auth or similar)
- Operation storage and retrieval
- Conflict resolution logic

**Phase 8: PWA Frontend**
- SvelteKit or React app
- IndexedDB for offline storage
- Service worker for background sync
- Web Push for notifications

**Phase 9: Sync Implementation**
- Client-server operation sync
- Snapshot bootstrap for new devices
- Conflict resolution (last-write-wins initially)
- Background sync on connectivity change

**Phase 10: Polish & Optimization**
- Performance optimization (large task lists)
- Accessibility improvements
- Mobile responsiveness
- Error handling and recovery

---

## Files Modified Summary

| File | Tasks | Lines Changed | Risk |
|------|-------|---------------|------|
| `crates/todomrs-core/src/parser.rs` | 1, 2 | ~20 lines | Low |
| `crates/todomrs-core/src/domain.rs` | 2 | ~15 lines | Low |
| `crates/todomrs-store/src/project_store.rs` | 2 | ~10 lines | Low |
| `crates/todomrs-tui/src/app.rs` | 2, 3, 4, 5, 6, 7 | ~250 lines | Medium |
| `crates/todomrs-tui/src/ui.rs` | 3, 4, 5, 6, 7, 8 | ~100 lines | Low |
| `crates/todomrs-tui/src/main.rs` | 2 | ~5 lines | High |

**Total:** ~400 lines of changes across 6 files

---

## Estimated Effort

- **Task 1** (Weekday bug): 30 minutes
- **Task 6** (Input navigation): 1 hour
- **Task 3** (Edit task): 1.5 hours
- **Task 4** (Search): 1.5 hours
- **Task 5** (Completed view): 1 hour
- **Task 2** (+project): 2 hours
- **Task 7** (Sidebar projects): 1 hour
- **Task 8** (Help text): 15 minutes

**Total:** ~8.5 hours (1 work day)

**Buffer:** Add 20% for debugging and testing = ~10 hours total

---

## Implementation Checklist

Use this checklist to track progress:

### Task 1: Fix Weekday Resolution Bug
- [ ] Fix next_weekday function in parser.rs
- [ ] Add new tests for weekday resolution
- [ ] Run `cargo test --package todomrs-core`
- [ ] Manual verification of weekday behavior
- [ ] Commit changes

### Task 6: Input Field Navigation
- [ ] Add cursor_position field to App
- [ ] Update Editing mode to use cursor position
- [ ] Add Left/Right/Home/End key handlers
- [ ] Update cursor rendering in ui.rs
- [ ] Run `cargo build`
- [ ] Manual testing of cursor navigation
- [ ] Commit changes

### Task 3: Edit Task
- [ ] Add EditingTask variant to InputMode
- [ ] Bind 'e' key in Normal mode
- [ ] Handle EditingTask mode in event handler
- [ ] Add update_task_title method
- [ ] Update UI prompts for EditingTask mode
- [ ] Run `cargo build`
- [ ] Manual testing of task editing
- [ ] Commit changes

### Task 4: Search Functionality
- [ ] Add Searching variant to InputMode
- [ ] Add search_query field to App
- [ ] Bind '/' key in Normal mode
- [ ] Handle Searching mode in event handler
- [ ] Update filtered_tasks to apply search filter
- [ ] Update UI to show search query and indicator
- [ ] Run `cargo build`
- [ ] Manual testing of search
- [ ] Commit changes

### Task 5: Completed/Archive View
- [ ] Add Completed variant to View enum
- [ ] Update filtered_tasks for Completed view
- [ ] Bind '5' key for Completed view
- [ ] Bind 'C' key to clear completed
- [ ] Add clear_completed method
- [ ] Update sidebar to include Completed
- [ ] Update status bar to include 'C' shortcut
- [ ] Run `cargo build`
- [ ] Manual testing of completed view
- [ ] Commit changes

### Task 2: Wire Up +project Auto-Create
- [ ] Add ProjectStore::find_by_name method
- [ ] Add Project::new constructor
- [ ] Modify parser return type
- [ ] Update parser tests
- [ ] Inject ProjectStore into App
- [ ] Wire up project auto-creation in create_task_from_input
- [ ] Run `cargo test`
- [ ] Manual testing of project creation
- [ ] Commit changes

### Task 7: Sidebar Project Display
- [ ] Add project_counts field to App
- [ ] Add refresh_project_counts method
- [ ] Call refresh_project_counts in refresh_tasks
- [ ] Update sidebar to show projects with counts
- [ ] Run `cargo build`
- [ ] Manual testing of sidebar projects
- [ ] Commit changes

### Task 8: Update Help Text
- [ ] Update help overlay with all new shortcuts
- [ ] Organize help by category
- [ ] Run `cargo build`
- [ ] Manual testing of help overlay
- [ ] Commit changes

### Final Validation
- [ ] Run `cargo test` (all packages)
- [ ] Run `cargo build --release`
- [ ] Complete manual testing checklist
- [ ] Verify all success criteria met
- [ ] Create summary commit/tag

---

## Notes

- **Commit frequency:** Commit after each task for easy rollback
- **Testing:** Run `cargo test` after each task to catch regressions early
- **Manual testing:** Essential for TUI features, cannot be fully automated
- **Documentation:** Update this plan if implementation deviates significantly
- **Time tracking:** Track actual time vs estimated time for future planning

---

**Plan created:** 2026-06-11  
**Planned completion:** After 1 work day of implementation  
**Next review:** After all tasks complete
