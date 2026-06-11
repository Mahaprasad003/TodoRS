# Phase 6.5: TUI Polish - Implementation Plan

## Overview
Fix critical UX gaps in the TUI before backend/sync work. This phase prioritizes daily-driver usability improvements.

---

## Task 1: Fix Weekday Resolution Bug

### File: `crates/todomrs-core/src/parser.rs`

**Current Issue:**
```rust
// Lines 280-287
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

The `+ 1` at the end always adds at least 1 day, preventing "today" as a valid result.

**Required Behavior:**
- `friday` on Monday → this Friday (4 days)
- `friday` on Saturday → next Friday (6 days)  
- `friday` on Friday → next Friday (7 days)
- General rule: if target weekday hasn't passed yet this week, return it; if it has passed or is today, return next week's occurrence

**Fix:**
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

**Tests to Update:**
- `test_next_weekday_same_day_is_next_week` (line 401-406) - already correct, verify still passes
- `test_next_weekday_next_day` (line 408-412) - verify still passes
- Add new test: `test_next_weekday_same_day_returns_seven_days` to explicitly verify Monday→Monday = 7 days

**Acceptance:**
- All existing tests pass
- New test verifies same-day returns 7 days
- Manual test: on Monday, `friday` returns this Friday, not next Friday

---

## Task 2: Wire Up +project Auto-Create

### Files to Modify:
- `crates/todomrs-tui/src/app.rs`
- `crates/todomrs-tui/src/main.rs` (to inject ProjectStore)

**Changes in `main.rs`:**
```rust
// Around line 40-50, add ProjectStore initialization
let project_store = ProjectStore::new(&db).await?;

// Pass to App::new
let app = App::new(
    user_id,
    device_id,
    task_store,
    op_store,
    project_store,  // NEW
);
```

**Changes in `app.rs`:**

1. Add `project_store` field to App struct (line 22-39):
```rust
pub struct App {
    // ... existing fields ...
    pub project_store: ProjectStore,  // NEW
}
```

2. Update `App::new` signature (line 61-79):
```rust
pub fn new(
    user_id: Uuid,
    device_id: Uuid,
    task_store: TaskStore,
    op_store: OperationStore,
    project_store: ProjectStore,  // NEW
) -> Self {
    Self {
        // ... existing fields ...
        project_store,  // NEW
    }
}
```

3. Update `create_task_from_input` (line 117-134):
```rust
async fn create_task_from_input(&mut self) -> Result<()> {
    let input = self.input_buffer.trim().to_string();
    if input.is_empty() {
        return Ok(());
    }

    let (mut task, _recurrence_rule) =
        NaturalLanguageParser::create_task_from_input(&input, self.user_id);

    // NEW: Handle project assignment
    if let Some(project_name) = /* parse project from input */ {
        // Look up or create project
        let project = self.project_store.find_by_name(self.user_id, &project_name).await?;
        let project_id = match project {
            Some(p) => p.id,
            None => {
                let new_project = Project::new(self.user_id, project_name.clone());
                self.project_store.create(&new_project).await?;
                new_project.id
            }
        };
        task.project_id = Some(project_id);
    }

    // ... rest of existing code ...
}
```

**Note:** Need to extract project name from parsed input. The parser already extracts `+project` into `parsed.project`, but `create_task_from_input` doesn't expose it. Options:
1. Change parser to return parsed.project alongside task
2. Re-parse input in app.rs to extract project
3. Add project as parameter to create_task_from_input

**Recommended:** Option 1 - modify parser to return `ParsedTask` struct alongside `Task`:
```rust
pub fn create_task_from_input(input: &str, user_id: Uuid) -> (Task, Option<RecurrenceRule>, Option<String>) {
    // ... existing code ...
    (task, recurrence_rule, parsed.project)
}
```

**Tests:**
- Add integration test: creating task with `+newproject` creates the project
- Add integration test: creating second task with `+existingproject` reuses the project
- Verify project_id is set correctly on task

**Acceptance:**
- `task +project` creates project if not exists, sets task.project_id
- `another task +project` reuses existing project
- Projects appear in database with correct user_id

---

## Task 3: Edit Task (e key)

### File: `crates/todomrs-tui/src/app.rs`

**Changes:**

1. Add new InputMode variant (line 13-16):
```rust
pub enum InputMode {
    Normal,
    Editing,
    EditingTask(usize),  // NEW: stores task index being edited
}
```

2. Bind 'e' key in Normal mode (around line 100):
```rust
KeyCode::Char('e') if key.modifiers.is_empty() => {
    let filtered = self.filtered_tasks();
    if self.selected_index < filtered.len() {
        let task = filtered[self.selected_index];
        self.input_buffer = task.title.clone();
        self.input_mode = InputMode::EditingTask(self.selected_index);
    }
}
```

3. Handle EditingTask mode in event handler (line 115-150):
```rust
InputMode::EditingTask(task_idx) => match key.code {
    KeyCode::Esc => {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
    }
    KeyCode::Enter => {
        self.update_task_title(task_idx).await?;
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
}
```

4. Add update_task_title method:
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

    // Record update operation for sync
    let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
    let op = Operation {
        op_id: Uuid::new_v4(),
        user_id: self.user_id,
        device_id: self.device_id,
        seq,
        entity: todomrs_sync::operations::Entity::Task,
        entity_id: task.id,
        op_type: todomrs_sync::operations::OperationType::Update,
        payload: todomrs_sync::operations::OperationPayload::TaskUpdate {
            title: Some(new_title.clone()),
            description: None,
            status: None,
            project_id: None,
            tag_ids: None,
            priority: None,
            due_at: None,
            scheduled_at: None,
            recurrence_rule_id: None,
            completed_at: None,
        },
        created_at: chrono::Utc::now(),
        synced_at: None,
    };
    self.op_store.append(&op).await?;

    self.status_message = Some(format!("Updated: {} → {}", old_title, new_title));
    self.refresh_tasks().await?;
    Ok(())
}
```

5. Update UI to show different prompt for EditingTask mode:
In `ui.rs`, update `draw_input_field` (line 187-212):
```rust
let title = match app.input_mode {
    InputMode::Normal => "Press 'a' to add task",
    InputMode::Editing => "Add task (Enter to save, Esc to cancel)",
    InputMode::EditingTask(_) => "Edit task (Enter to save, Esc to cancel)",
};
```

**Tests:**
- Manual test: select task, press 'e', edit title, press Enter, verify update
- Verify operation is recorded for sync
- Verify Esc cancels edit

**Acceptance:**
- 'e' key enters edit mode with current task title
- Enter saves changes, updates database, records operation
- Esc cancels without changes
- Status message shows old → new title

---

## Task 4: Search Functionality (/ key)

### File: `crates/todomrs-tui/src/app.rs`

**Changes:**

1. Add Search mode and state (line 13-16, 22-39):
```rust
pub enum InputMode {
    Normal,
    Editing,
    EditingTask(usize),
    Searching,  // NEW
}

pub struct App {
    // ... existing fields ...
    pub search_query: String,  // NEW
    pub previous_view: Option<View>,  // NEW: restore view after search
}
```

2. Initialize new fields in `App::new` (line 61-79):
```rust
Self {
    // ... existing fields ...
    search_query: String::new(),
    previous_view: None,
}
```

3. Bind '/' key in Normal mode (around line 100):
```rust
KeyCode::Char('/') if key.modifiers.is_empty() => {
    self.previous_view = Some(self.current_view.clone());
    self.input_mode = InputMode::Searching;
    self.search_query.clear();
}
```

4. Handle Searching mode (line 115-150):
```rust
InputMode::Searching => match key.code {
    KeyCode::Esc => {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
        if let Some(view) = self.previous_view.take() {
            self.current_view = view;
        }
    }
    KeyCode::Enter => {
        self.input_mode = InputMode::Normal;
        // Keep search_query active for filtering
    }
    KeyCode::Char(c) => {
        self.search_query.push(c);
    }
    KeyCode::Backspace => {
        self.search_query.pop();
    }
    _ => {}
}
```

5. Update `filtered_tasks` to apply search filter (line 87-100):
```rust
pub fn filtered_tasks(&self) -> Vec<&Task> {
    let today = chrono::Utc::now().date_naive();
    let mut tasks: Vec<&Task> = match self.current_view {
        View::Inbox => self.tasks.iter().collect(),
        View::Today => self.tasks.iter()
            .filter(|t| t.due_at.map(|dt| dt.date_naive() == today).unwrap_or(false))
            .collect(),
        View::Upcoming => self.tasks.iter()
            .filter(|t| t.due_at.map(|dt| dt.date_naive() > today).unwrap_or(false))
            .collect(),
        View::Projects => Vec::new(),
    };

    // Apply search filter if active
    if !self.search_query.is_empty() {
        let query = self.search_query.to_lowercase();
        tasks.retain(|t| t.title.to_lowercase().contains(&query));
    }

    tasks
}
```

6. Update UI to show search query:
In `ui.rs`, update `draw_input_field`:
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

let display_text = match app.input_mode {
    InputMode::Searching => &app.search_query,
    _ => &app.input_buffer,
};
```

7. Show search indicator in main content area:
In `draw_main_content`, add search badge if search is active:
```rust
let title = if !app.search_query.is_empty() {
    format!("{} [search: {}]", title, app.search_query)
} else {
    title.to_string()
};
```

**Tests:**
- Manual test: press '/', type query, verify filtered results
- Verify Esc clears search and restores view
- Verify case-insensitive matching

**Acceptance:**
- '/' enters search mode
- Typing filters tasks by title substring (case-insensitive)
- Enter confirms search (stays filtered)
- Esc clears search and restores previous view
- Search indicator shown in UI

---

## Task 5: Completed/Archive View

### Files to Modify:
- `crates/todomrs-tui/src/app.rs`
- `crates/todomrs-tui/src/ui.rs`

**Changes in `app.rs`:**

1. Add View::Completed variant (line 8-13):
```rust
pub enum View {
    Inbox,
    Today,
    Upcoming,
    Projects,
    Completed,  // NEW
}
```

2. Update `filtered_tasks` (line 87-100):
```rust
pub fn filtered_tasks(&self) -> Vec<&Task> {
    let today = chrono::Utc::now().date_naive();
    let mut tasks: Vec<&Task> = match self.current_view {
        View::Inbox => self.tasks.iter().collect(),
        View::Today => self.tasks.iter()
            .filter(|t| t.due_at.map(|dt| dt.date_naive() == today).unwrap_or(false))
            .collect(),
        View::Upcoming => self.tasks.iter()
            .filter(|t| t.due_at.map(|dt| dt.date_naive() > today).unwrap_or(false))
            .collect(),
        View::Projects => Vec::new(),
        View::Completed => self.tasks.iter()
            .filter(|t| t.status == TaskStatus::Completed && t.status != TaskStatus::Deleted)
            .collect(),  // NEW
    };
    // ... rest of code ...
}
```

3. Bind '5' key in Normal mode (around line 100):
```rust
KeyCode::Char('5') => {
    self.current_view = View::Completed;
    self.selected_index = 0;
}
```

4. Add 'C' key to clear all completed (around line 100):
```rust
KeyCode::Char('C') if key.modifiers.is_empty() => {
    self.clear_completed().await?
}
```

5. Add clear_completed method:
```rust
async fn clear_completed(&mut self) -> Result<()> {
    let completed: Vec<Task> = self.tasks.iter()
        .filter(|t| t.status == TaskStatus::Completed)
        .cloned()
        .collect();

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

    self.status_message = Some(format!("Cleared {} completed tasks", /* count */));
    self.selected_index = 0;
    self.refresh_tasks().await?;
    Ok(())
}
```

**Changes in `ui.rs`:**

1. Update sidebar to include Completed (line 36-50):
```rust
let items = vec![
    ListItem::new("Inbox"),
    ListItem::new("Today"),
    ListItem::new("Upcoming"),
    ListItem::new("Projects"),
    ListItem::new("Completed"),  // NEW
];

let selected = match app.current_view {
    View::Inbox => 0,
    View::Today => 1,
    View::Upcoming => 2,
    View::Projects => 3,
    View::Completed => 4,  // NEW
};
```

2. Update main content title (line 62-70):
```rust
let title = match app.current_view {
    View::Inbox => "Inbox",
    View::Today => "Today",
    View::Upcoming => "Upcoming",
    View::Projects => "Projects",
    View::Completed => "Completed",  // NEW
};
```

3. Update status bar to include 'C' shortcut (line 231-252):
```rust
let status = Line::from(vec![
    // ... existing items ...
    Span::styled("C", Style::default().fg(Color::Yellow)),
    Span::raw(" Clear "),
]);
```

4. Update help overlay (line 254-290):
```rust
let help_text = vec![
    // ... existing items ...
    Line::from("5  — Completed view"),
    Line::from("C  — Clear all completed"),
    Line::from("/  — Search"),
    Line::from("e  — Edit task"),
];
```

**Tests:**
- Verify '5' switches to Completed view
- Verify only completed tasks shown
- Verify 'C' deletes all completed tasks
- Verify operations recorded for sync

**Acceptance:**
- '5' key switches to Completed view
- Only completed (non-deleted) tasks shown
- 'C' key clears all completed tasks with confirmation message
- Operations recorded for sync

---

## Task 6: Input Field Navigation

### File: `crates/todomrs-tui/src/app.rs`

**Changes:**

1. Add cursor position tracking (line 22-39):
```rust
pub struct App {
    // ... existing fields ...
    pub cursor_position: usize,  // NEW: track cursor in input buffer
}
```

2. Initialize in `App::new`:
```rust
Self {
    // ... existing fields ...
    cursor_position: 0,
}
```

3. Reset cursor when entering edit modes:
In all mode transitions to Editing/EditingTask/Searching:
```rust
self.cursor_position = self.input_buffer.len();
```

4. Update Char/Backspace handling to maintain cursor position:
```rust
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
```

5. Add navigation keys in all editing modes:
```rust
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
KeyCode::Home | KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => {
    self.cursor_position = 0;
}
KeyCode::End | KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => {
    self.cursor_position = self.input_buffer.len();
}
KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
    // Delete word backwards
    let before_cursor = &self.input_buffer[..self.cursor_position];
    let last_space = before_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);
    self.input_buffer.drain(last_space..self.cursor_position);
    self.cursor_position = last_space;
}
```

6. Update UI cursor rendering:
In `ui.rs`, update `draw_input_field` (line 187-212):
```rust
if matches!(app.input_mode, InputMode::Editing | InputMode::EditingTask(_) | InputMode::Searching) {
    let display_text = match app.input_mode {
        InputMode::Searching => &app.search_query,
        _ => &app.input_buffer,
    };
    let cursor_pos = app.cursor_position.min(display_text.len());
    let cursor_x = area.x + (cursor_pos as u16).min(area.width.saturating_sub(2)) + 1;
    f.set_cursor(cursor_x, area.y + 1);
}
```

**Note:** For Searching mode, also need to add `search_cursor_position` field and apply same logic.

**Tests:**
- Manual test: arrow keys move cursor
- Ctrl+A moves to start, Ctrl+E to end
- Ctrl+W deletes word backwards
- Cursor position maintained across edits

**Acceptance:**
- Left/Right arrows navigate cursor
- Ctrl+A (Home) moves to start
- Ctrl+E (End) moves to end
- Ctrl+W deletes word backwards
- Cursor renders at correct position

---

## Task 7: Sidebar Project Display

### Files to Modify:
- `crates/todomrs-tui/src/app.rs`
- `crates/todomrs-tui/src/ui.rs`

**Changes in `app.rs`:**

1. Add project task counts field (line 22-39):
```rust
pub struct App {
    // ... existing fields ...
    pub project_counts: HashMap<Uuid, (String, usize)>,  // NEW: project_id -> (name, count)
}
```

2. Initialize in `App::new`:
```rust
Self {
    // ... existing fields ...
    project_counts: HashMap::new(),
}
```

3. Add method to refresh project counts:
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

4. Call in `refresh_tasks`:
```rust
pub async fn refresh_tasks(&mut self) -> Result<()> {
    self.tasks = self.task_store.get_all(self.user_id).await?;
    self.refresh_project_counts().await?;  // NEW
    // ... rest of code ...
}
```

**Changes in `ui.rs`:**

1. Update sidebar to show projects with counts (line 36-50):
```rust
fn draw_sidebar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(2 + app.project_counts.len() as u16)])
        .split(area);

    // Views section
    let items = vec![/* existing items */];
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Views"));
    // ... render list ...

    // Projects section
    let project_items: Vec<ListItem> = app.project_counts.values()
        .map(|(name, count)| ListItem::new(format!("{} ({})", name, count)))
        .collect();

    let project_list = List::new(project_items)
        .block(Block::default().borders(Borders::ALL).title("Projects"));
    f.render_widget(project_list, chunks[1]);
}
```

**Note:** Need to import HashMap in app.rs:
```rust
use std::collections::HashMap;
```

**Tests:**
- Verify project counts update when tasks added/removed
- Verify projects without tasks still shown (with count 0)
- Verify sidebar updates after task operations

**Acceptance:**
- Sidebar shows project names with task counts
- Counts update when tasks change
- Projects section separated from views by divider
- Empty projects shown with (0) count

---

## Task 8: Update Help Text

### File: `crates/todomrs-tui/src/ui.rs`

**Changes:**

Update `draw_help` function (line 254-290) to include all new shortcuts:
```rust
let help_text = vec![
    Line::from(Span::styled("Keyboard Shortcuts", Style::default().add_modifier(Modifier::BOLD))),
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
    Line::from("  C      — Clear all completed"),
    Line::from(""),
    Line::from("Search & Navigation:"),
    Line::from("  /      — Search"),
    Line::from("  ?      — Toggle help"),
    Line::from("  q      — Quit"),
    Line::from(""),
    Line::from("Input Mode:"),
    Line::from("  ←/→    — Move cursor"),
    Line::from("  Ctrl+A — Start of line"),
    Line::from("  Ctrl+E — End of line"),
    Line::from("  Ctrl+W — Delete word"),
    Line::from("  Enter  — Confirm"),
    Line::from("  Esc    — Cancel"),
];
```

**Acceptance:**
- Help overlay shows all shortcuts organized by category
- All new shortcuts documented
- Clear, readable layout

---

## Testing Strategy

### Unit Tests
- Parser: weekday resolution with various day combinations
- Parser: project extraction from input
- App: edit task flow
- App: search filtering logic
- App: completed view filtering

### Integration Tests
- Create task with +project, verify project created
- Edit task, verify database updated and operation recorded
- Search for task, verify correct filtering
- Clear completed, verify all deleted and operations recorded

### Manual Testing Checklist
- [ ] Monday: `friday` → this Friday (4 days)
- [ ] Saturday: `friday` → next Friday (6 days)
- [ ] Friday: `friday` → next Friday (7 days)
- [ ] Create `task +newproject`, verify project exists
- [ ] Create `task2 +newproject`, verify same project reused
- [ ] Select task, press 'e', edit title, Enter, verify updated
- [ ] Press '/', type query, verify filtered results
- [ ] Press Esc, verify search cleared and view restored
- [ ] Press '5', verify completed view shown
- [ ] Press 'C', verify all completed tasks cleared
- [ ] Arrow keys in input, verify cursor moves
- [ ] Ctrl+A/E/W, verify expected behavior
- [ ] Sidebar shows projects with counts
- [ ] Add/remove tasks, verify counts update
- [ ] Press '?', verify help shows all shortcuts

---

## Dependencies & Order

**Recommended Implementation Order:**
1. **Task 1** (Weekday bug) - Foundation fix, no dependencies
2. **Task 6** (Input navigation) - Needed for Tasks 3, 4, 5
3. **Task 3** (Edit task) - Depends on Task 6 for cursor handling
4. **Task 4** (Search) - Independent, can be done in parallel
5. **Task 5** (Completed view) - Independent, can be done in parallel
6. **Task 2** (+project) - Requires ProjectStore injection, more complex
7. **Task 7** (Sidebar projects) - Depends on Task 2
8. **Task 8** (Help text) - Last, after all features complete

**Critical Path:** Tasks 1 → 6 → 3

---

## Risks & Considerations

### High Risk
1. **ProjectStore injection** (Task 2): Requires changes to main.rs initialization flow. Must ensure database migrations exist for projects table.
2. **Operation recording**: All mutations must record operations for future sync. Easy to miss one.

### Medium Risk
1. **Cursor position tracking**: Must handle edge cases (empty buffer, cursor at boundaries).
2. **Search state management**: Need to properly restore view after search cancelled.

### Low Risk
1. **Weekday calculation**: Well-tested, straightforward math fix.
2. **UI updates**: Ratatui is declarative, low risk of state corruption.

### Mitigation
- Run `cargo test` after each task
- Manual testing after each feature
- Commit after each task for easy rollback

---

## Success Criteria

Phase 6.5 is complete when:
1. ✅ Weekday resolution matches Todoist/Things behavior
2. ✅ +project auto-creates and reuses projects
3. ✅ Can edit task titles with 'e' key
4. ✅ Can search tasks with '/' key
5. ✅ Can view and clear completed tasks
6. ✅ Input field has full navigation (arrows, Ctrl+A/E/W)
7. ✅ Sidebar shows project counts
8. ✅ Help text documents all shortcuts
9. ✅ All operations recorded for future sync
10. ✅ No regressions in existing functionality

---

## Post-Phase 6.5 Priorities

**Updated Phase Ordering (recommendation):**
- **Phase 7**: Backend API (delayed until TUI is daily-driver ✅)
- **Phase 8**: PWA frontend (depends on Phase 7)
- **Phase 9**: Sync implementation (depends on Phase 7)
- **Phase 10**: Polish & optimization

**Rationale:** The TUI now has sufficient UX for daily use. Backend API should be next to enable multi-device sync, which is the primary value proposition. PWA can be built in parallel once API exists.

---

## Files Modified Summary

| File | Tasks | Lines Changed |
|------|-------|---------------|
| `crates/todomrs-core/src/parser.rs` | 1 | ~15 lines |
| `crates/todomrs-tui/src/app.rs` | 2, 3, 4, 5, 6, 7 | ~250 lines |
| `crates/todomrs-tui/src/ui.rs` | 3, 4, 5, 6, 7, 8 | ~100 lines |
| `crates/todomrs-tui/src/main.rs` | 2 | ~5 lines |

**Total:** ~370 lines of changes across 4 files

---

## Estimated Effort

- **Task 1** (Weekday bug): 30 minutes
- **Task 2** (+project): 2 hours
- **Task 3** (Edit task): 1.5 hours
- **Task 4** (Search): 1.5 hours
- **Task 5** (Completed view): 1 hour
- **Task 6** (Input navigation): 1 hour
- **Task 7** (Sidebar projects): 1 hour
- **Task 8** (Help text): 15 minutes

**Total:** ~8 hours (1 work day)
