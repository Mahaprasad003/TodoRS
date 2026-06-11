# Phase 6.5: TUI Polish — Final Implementation Plan

> **Status:** Reviewed and corrected  
> **Total effort:** ~10 hours (1.5 work days including buffer)  
> **Files modified:** 6 files, ~400 lines  

---

## Implementation Order (Critical Path)

```
Task 1 (Weekday bug) ─┐
                       ├── Task 6 (Input nav) ──→ Task 3 (Edit task) ──┐
Task 4 (Search) ──────┤                                                 │
Task 5 (Completed) ───┘                                                 │
                                                                        ├──→ Task 2 (+project) ──→ Task 7 (Sidebar) ──→ Task 8 (Help)
                                                                        │
                                                                        └── (Task 6 also needed by 3, 4, 5 for cursor)
```

---

## Key Fixes Identified by Reviewer

The following **blocker issues** were identified in the original plan and must be applied during implementation:

### 🔴 Fix 1: Use `task_id: Uuid` not `usize` for EditingTask
**Instead of:**
```rust
enum InputMode { EditingTask(usize) }
```
**Use:**
```rust
enum InputMode { EditingTask(Uuid) }
```
Store `task.id` (stable) instead of `filtered_tasks()` index (stale-prone).

### 🔴 Fix 2: Split Home/End and Ctrl+A/E into separate arms
**Instead of:**
```rust
KeyCode::Home | KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => ...
```
**Use:**
```rust
KeyCode::Home => { self.cursor_position = 0; }
KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => { self.cursor_position = 0; }
```
Same for `End` / `Ctrl+E`.

### 🔴 Fix 3: Replace `TaskStatus::Deleted` with `deleted_at.is_none()`
`TaskStatus` only has `Pending` | `Completed`. Use `task.deleted_at.is_none()` in filters.

### 🔴 Fix 4: Add `ProjectStore::find_by_name` to project_store.rs
This method doesn't exist yet. ADD before referencing in app.rs.

### 🔴 Fix 5: Add `Project::new` constructor to domain.rs
`Project` has no `new()` method. ADD before wiring up +project.

### 🔴 Fix 6: Add missing imports
- `crossterm::event::KeyModifiers` in app.rs
- `std::collections::HashMap` in app.rs
- `ProjectStore` in app.rs

### 🔴 Fix 7: Add `View::Completed` arm in `draw_status_bar`
The `view_name` match in `draw_status_bar` will be non-exhaustive without it.

### 🔴 Fix 8: Clear `previous_view` on manual view switch
Add `self.previous_view = None;` in view-switching arms (keys 1-5).

### 🔴 Fix 9: Help overlay dimensions
Increase from `.max(14)`/`.max(30)` to `.max(24)`/`.max(50)`.

### 🟡 Fix 10: Use `Operation::update_task_title` helper
Don't inline the full struct — use existing helper for consistency.

### 🟡 Fix 11: Track byte index for cursor_position (or document ASCII-only)
`String::insert/remove` use byte indices. For emoji/accented chars, `cursor_position` must track byte index. Either compute `cursor_position += c.len_utf8()` or document as ASCII-only.

---

## Task Breakdown

### Task 1: Fix Weekday Resolution Bug
**File:** `crates/todomrs-core/src/parser.rs` (lines 280-287)  
**Effort:** 30 min | **Risk:** Low

Replace `next_weekday()`:
```rust
pub fn next_weekday(from: NaiveDate, weekday: Weekday) -> NaiveDate {
    let from_weekday = from.weekday();
    let target_day = weekday.num_days_from_monday() as i32;
    let current_day = from_weekday.num_days_from_monday() as i32;

    if target_day == current_day {
        return from + Duration::days(7);
    }

    let days_ahead = (target_day - current_day + 7) % 7;
    from + Duration::days(days_ahead as i64)
}
```

**Add tests** (after line 418):
- `test_next_weekday_same_day_returns_seven_days` (Mon→Mon = 7 days)
- `test_next_weekday_this_week` (Mon→Fri = 4 days, Mon→Wed = 2 days)

**Acceptance:** All existing + new tests pass. Friday on Friday = 7 days.

---

### Task 2: Wire Up +project Auto-Create
**Files:** `domain.rs`, `parser.rs`, `project_store.rs`, `app.rs`, `main.rs`  
**Effort:** 2.5 hr | **Risk:** Medium

**2a. Add `Project::new` to `domain.rs`:**
```rust
impl Project {
    pub fn new(user_id: Uuid, name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(), user_id, name,
            color: None, sort_order: 0,
            created_at: now, updated_at: now, archived_at: None,
        }
    }
}
```

**2b. Add `find_by_name` to `project_store.rs`:**
```rust
pub async fn find_by_name(&self, user_id: Uuid, name: &str) -> Result<Option<Project>> {
    let row: Option<ProjectRow> = sqlx::query_as(
        "SELECT * FROM projects WHERE user_id = ? AND name = ? AND archived_at IS NULL"
    )
    .bind(user_id).bind(name)
    .fetch_optional(&self.pool).await?;
    Ok(row.map(ProjectRow::into_project))
}
```

**2c. Modify parser return type** (or extract project directly in app.rs):
If modifying parser: change `create_task_from_input` to return `(Task, Option<RecurrenceRule>, Option<String>)`.
**Alternative (simpler, recommended):** In `app.rs`, call `NaturalLanguageParser::parse(&input)` to get `parsed.project`, then call `create_task_from_input` as before. Avoids breaking the parser API.

**2d. Inject ProjectStore:**
- `main.rs`: Add `let project_store = ProjectStore::new(db.pool().clone());`, pass to `App::new`
- `app.rs`: Add `project_store: ProjectStore` field, update constructor signature

**2e. Create/assign project in `create_task_from_input`:**
```rust
let (mut task, _recurrence_rule) =
    NaturalLanguageParser::create_task_from_input(&input, self.user_id);

if let Some(project_name) = NaturalLanguageParser::parse(&input).project {
    let existing = self.project_store.find_by_name(self.user_id, &project_name).await?;
    let project_id = match existing {
        Some(p) => p.id,
        None => {
            let new_project = Project::new(self.user_id, project_name);
            self.project_store.create(&new_project).await?;
            // Record project creation operation...
            new_project.id
        }
    };
    task.project_id = Some(project_id);
}
```

**Acceptance:** `task +proj` creates project. `task2 +proj` reuses it. Operations recorded.

---

### Task 3: Edit Task (e key)
**Files:** `app.rs`, `ui.rs`  
**Effort:** 1.5 hr | **Risk:** Low

**Key decisions (reviewer corrections applied):**
- `InputMode::EditingTask(Uuid)` — store task.id, not index
- Use `Operation::update_task_title` helper (exists in sync crate)

**3a. Add variant + field:**
```rust
pub enum InputMode {
    Normal,
    Editing,
    EditingTask(Uuid),  // stores task.id being edited
}
```

**3b. Bind 'e' key:**
```rust
KeyCode::Char('e') if key.modifiers.is_empty() => {
    let filtered = self.filtered_tasks();
    if self.selected_index < filtered.len() {
        let task = filtered[self.selected_index];
        self.input_buffer = task.title.clone();
        self.cursor_position = self.input_buffer.len();
        self.input_mode = InputMode::EditingTask(task.id);
    }
}
```

**3c. Handle EditingTask in event handler** — Esc cancels, Enter saves via `update_task_title`.

**3d. `update_task_title` method:**
- Look up task by ID (not index) in `self.tasks`
- Use `Operation::update_task_title` helper for operation recording

**3e. UI:** Show "Edit task (Enter to save, Esc to cancel)" prompt.

---

### Task 4: Search Functionality (/ key)
**Files:** `app.rs`, `ui.rs`  
**Effort:** 1.5 hr | **Risk:** Low

**4a. Add `InputMode::Searching` variant, `search_query` and `previous_view` fields.**

**4b. Bind '/' key:** Save current view, enter searching mode.
**Bind '1-5' keys:** Add `self.previous_view = None;` to prevent stale restore.

**4c. Update `filtered_tasks()`:** After view filtering, apply `search_query` as case-insensitive substring match:
```rust
if !self.search_query.is_empty() {
    let query = self.search_query.to_lowercase();
    tasks.retain(|t| t.title.to_lowercase().contains(&query));
}
```

**4d. UI:** Show search prompt, search query text, and search indicator in title bar.

---

### Task 5: Completed/Archive View
**Files:** `app.rs`, `ui.rs`  
**Effort:** 1 hr | **Risk:** Low

**5a. Add `View::Completed` to enum.**

**5b. Filter:** `t.status == TaskStatus::Completed && t.deleted_at.is_none()`

**5c. Bind '5'** to switch, **'C'** to clear all completed (soft-delete each).

**5d. UI updates:** Sidebar item at index 4, "Completed" title in main content, status bar includes `C` shortcut. **Must add `View::Completed` arm to ALL match statements** (sidebar, main content title, status bar view_name, filtered_tasks).

---

### Task 6: Input Field Navigation
**Files:** `app.rs`, `ui.rs`  
**Effort:** 1 hr | **Risk:** Low

**6a. Add `cursor_position: usize` field.**

**6b. Update Char to insert at cursor:**
```rust
KeyCode::Char(c) => {
    self.input_buffer.insert(self.cursor_position, c);
    self.cursor_position += c.len_utf8();  // byte-safe
}
```

**6c. Update Backspace to remove at cursor:**
```rust
KeyCode::Backspace => {
    if self.cursor_position > 0 {
        self.cursor_position -= 1;
        self.input_buffer.remove(self.cursor_position);
    }
}
```
> ⚠ `remove(i)` is O(n) and uses byte index. For char-safety on backspace, iterate backwards from `cursor_position` by `self.input_buffer[..cursor_position].chars().rev().next()` or use `self.cursor_position = self.input_buffer[..self.cursor_position-1].len()` to find previous char boundary. For ASCII-only this is fine.

**6d. Add:**
```rust
KeyCode::Left => { if self.cursor_position > 0 { self.cursor_position -= 1; } }
KeyCode::Right => { if self.cursor_position < self.input_buffer.len() { self.cursor_position += 1; } }
KeyCode::Home => { self.cursor_position = 0; }
KeyCode::End => { self.cursor_position = self.input_buffer.len(); }
KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => { self.cursor_position = 0; }
KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => { self.cursor_position = self.input_buffer.len(); }
```
> ⚠ Note: `Home`/`End` are split from `Ctrl+A`/`Ctrl+E` into separate arms (reviewer fix 2).

**6e. UI:** Set cursor position based on `app.cursor_position`.

---

### Task 7: Sidebar Project Display
**Files:** `app.rs`, `ui.rs`  
**Effort:** 1 hr | **Risk:** Low

**7a. Add `project_counts: HashMap<Uuid, (String, usize)>` field.**
**7b. Add `refresh_project_counts()` method**, call from `refresh_tasks()`.
**7c. UI:** Split sidebar into two sections (Views + Projects) using `Layout::default().direction(Direction::Vertical)`.

---

### Task 8: Update Help Text
**File:** `ui.rs`  
**Effort:** 15 min | **Risk:** Low

**Updated overlay** (organized by category, increased dimensions):
```rust
let help_h = (area.height / 2).min(area.height.saturating_sub(2)).max(24);
let help_w = (area.width / 2).min(area.width.saturating_sub(4)).max(50);
```

Content: Navigation (1-5, j/k), Task Operations (a, e, x, d, /, C, ?, q), Input Mode (arrows, Home, End, Enter, Esc).

---

## Testing Strategy

| Phase | Scope | Command |
|-------|-------|---------|
| After each task | Unit tests | `cargo test --package <crate>` |
| After all tasks | Full suite | `cargo test` |
| Final validation | Build check | `cargo build --release` |
| Manual | TUI behavior | Run `cargo run --bin todomrs` |

### Manual Test Checklist
- [ ] `friday` on Mon → 4 days; on Fri → 7 days; on Sat → 6 days
- [ ] `task +proj` creates project; `task2 +proj` reuses
- [ ] 'e' on task edits title; 'Esc' cancels
- [ ] '/' searches; 'Esc' clears; 'Enter' confirms
- [ ] '5' shows completed; 'C' clears them
- [ ] Arrows/Home/End/Ctrl+A/Ctrl+E navigate input cursor
- [ ] Sidebar shows project names with (counts)
- [ ] '?' shows all shortcuts organized by category
- [ ] All existing functionality still works (add, complete, delete, views 1-4)

---

## Success Criteria

1. ✅ Weekday resolution: same-day = 7 days (correct)
2. ✅ +project auto-creates and reuses projects
3. ✅ 'e' key edits task titles
4. ✅ '/' key searches (case-insensitive)
5. ✅ '5' view + 'C' clear completed works
6. ✅ Input navigation (arrows, Home, End, Ctrl+A, Ctrl+E)
7. ✅ Sidebar shows projects with counts
8. ✅ Help overlay documents all shortcuts
9. ✅ All operations recorded for sync
10. ✅ No regressions in existing tests/functionality

---

## Risk Register

| Risk | Severity | Mitigation |
|------|----------|------------|
| Stale EditingTask index | High | **Fixed:** Use `Uuid` not `usize` |
| Home/End match arm syntax | High | **Fixed:** Split into separate arms |
| Missing enum arm (Status::Deleted) | High | **Fixed:** Use `deleted_at.is_none()` |
| Missing methods/imports | High | **Fixed:** All identified in review |
| cursor_position byte vs char | Medium | Track char len or document ASCII limit |
| previous_view stale on view switch | Medium | **Fixed:** Clear on all 1-5 binds |
| Help overlay truncation | Low | **Fixed:** Increased dimensions |

---

## Files Modified Summary

| File | Tasks | Lines | Risk |
|------|-------|-------|------|
| `crates/todomrs-core/src/parser.rs` | 1 | ~15 | Low |
| `crates/todomrs-core/src/domain.rs` | 2 | ~15 | Low |
| `crates/todomrs-store/src/project_store.rs` | 2 | ~10 | Low |
| `crates/todomrs-tui/src/app.rs` | 2,3,4,5,6,7 | ~250 | Medium |
| `crates/todomrs-tui/src/ui.rs` | 3,4,5,6,7,8 | ~100 | Low |
| `crates/todomrs-tui/src/main.rs` | 2 | ~5 | Low |

**Total:** ~400 lines across 6 files

---

*Plan finalized: 2026-06-11*  
*Planner: builtin/planner → builtin/reviewer → corrected*  
*Next: User approval → implementation*
