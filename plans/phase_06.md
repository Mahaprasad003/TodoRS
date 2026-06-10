# Phase 6: TUI Task Views + Editing + Store Integration

## Session Goal

Connect the TUI to the database. Load real tasks from SQLite, display them in views, and implement task creation via natural language input. By the end of this session, you should have a working TUI that can create, display, and complete tasks.

## Expected Outcome

- TUI loads tasks from SQLite on startup
- Inbox view shows all tasks
- Today view shows tasks due today
- Quick add creates tasks via natural language parser
- Complete/uncomplete tasks with 'x' key
- Delete tasks with 'd' key
- Task selection and detail view
- All changes create operations in the operation log
- `cargo run --bin todomrs` is now a usable task manager

## Context

Phase 5 is complete. You have:
- TUI skeleton with layout and navigation
- Input mode for quick add
- Status bar and help overlay

Now you'll connect everything. The TUI will use the store layer to persist tasks and the parser to handle natural language input.

## Prerequisites

- Phase 5 complete and committed
- All stores working
- Parser and recurrence engine working
- Operation log working

## Tasks

### Task 1: Load Tasks from Database on Startup

**Objective:** Connect TUI to SQLite and load tasks on startup.

**Steps:**

1. Update `crates/todomrs-tui/src/main.rs`:

```rust
mod app;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use todomrs_store::{Database, TaskStore};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup database
    let db = Database::new("sqlite:./todomrs.db").await?;
    let task_store = TaskStore::new(db.pool().clone());

    // Create app
    let user_id = Uuid::new_v4(); // TODO: Load from config
    let device_id = Uuid::new_v4(); // TODO: Load from config

    // Ensure user exists
    sqlx::query("INSERT OR IGNORE INTO users (id, email) VALUES (?, ?)")
        .bind(user_id.to_string())
        .bind("local@todomrs")
        .execute(db.pool())
        .await?;

    let mut app = App::new(user_id, device_id, task_store);
    app.refresh_tasks().await?;

    // Main loop
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;
            app.handle_event(event).await?;
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    db.close().await;
    Ok(())
}
```

2. Update `crates/todomrs-tui/src/app.rs`:

```rust
use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use todomrs_core::domain::{Project, Task};
use todomrs_core::NaturalLanguageParser;
use todomrs_store::TaskStore;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Inbox,
    Today,
    Upcoming,
    Projects,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

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
}

impl App {
    pub fn new(user_id: Uuid, device_id: Uuid, task_store: TaskStore) -> Self {
        Self {
            should_quit: false,
            current_view: View::Inbox,
            selected_index: 0,
            tasks: Vec::new(),
            projects: Vec::new(),
            user_id,
            device_id,
            show_help: false,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            task_store,
        }
    }

    pub async fn refresh_tasks(&mut self) -> Result<()> {
        self.tasks = self.task_store.get_all(self.user_id).await?;
        Ok(())
    }

    pub fn filtered_tasks(&self) -> Vec<&Task> {
        match self.current_view {
            View::Inbox => self.tasks.iter().collect(),
            View::Today => {
                let today = chrono::Utc::now().date_naive();
                self.tasks
                    .iter()
                    .filter(|t| {
                        t.due_at
                            .map(|dt| dt.date_naive() == today)
                            .unwrap_or(false)
                    })
                    .collect()
            }
            View::Upcoming => {
                let today = chrono::Utc::now().date_naive();
                self.tasks
                    .iter()
                    .filter(|t| {
                        t.due_at
                            .map(|dt| dt.date_naive() > today)
                            .unwrap_or(false)
                    })
                    .collect()
            }
            View::Projects => Vec::new(), // TODO: Implement
        }
    }

    pub async fn handle_event(&mut self, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            match self.input_mode {
                InputMode::Normal => {
                    if self.show_help {
                        if let KeyCode::Char('?') | KeyCode::Esc = key.code {
                            self.show_help = false;
                        }
                        return Ok(());
                    }

                    match key.code {
                        KeyCode::Char('q') => self.should_quit = true,
                        KeyCode::Char('?') => self.show_help = true,
                        KeyCode::Char('a') => self.input_mode = InputMode::Editing,
                        KeyCode::Char('j') | KeyCode::Down => self.next_item(),
                        KeyCode::Char('k') | KeyCode::Up => self.previous_item(),
                        KeyCode::Char('x') => self.toggle_complete().await?,
                        KeyCode::Char('1') => { self.current_view = View::Inbox; self.selected_index = 0; }
                        KeyCode::Char('2') => { self.current_view = View::Today; self.selected_index = 0; }
                        KeyCode::Char('3') => { self.current_view = View::Upcoming; self.selected_index = 0; }
                        KeyCode::Char('4') => { self.current_view = View::Projects; self.selected_index = 0; }
                        _ => {}
                    }
                }
                InputMode::Editing => {
                    match key.code {
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
                    }
                }
            }
        }
        Ok(())
    }

    async fn create_task_from_input(&mut self) -> Result<()> {
        if self.input_buffer.trim().is_empty() {
            return Ok(());
        }

        let (mut task, _recurrence) =
            NaturalLanguageParser::create_task_from_input(&self.input_buffer, self.user_id);

        self.task_store.create(&task).await?;
        self.refresh_tasks().await?;

        Ok(())
    }

    async fn toggle_complete(&mut self) -> Result<()> {
        let filtered = self.filtered_tasks();
        if self.selected_index >= filtered.len() {
            return Ok(());
        }

        let mut task = filtered[self.selected_index].clone();
        if task.status == todomrs_core::domain::TaskStatus::Completed {
            task.status = todomrs_core::domain::TaskStatus::Pending;
            task.completed_at = None;
        } else {
            task.complete();
        }
        task.updated_at = chrono::Utc::now();

        self.task_store.update(&task).await?;
        self.refresh_tasks().await?;

        Ok(())
    }

    pub fn next_item(&mut self) {
        let len = self.filtered_tasks().len();
        if len > 0 && self.selected_index < len - 1 {
            self.selected_index += 1;
        }
    }

    pub fn previous_item(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }
}
```

3. Verify it compiles:
```bash
cargo build --bin todomrs
```

Expected: Compiles successfully.

**Commit:**
```bash
git add .
git commit -m "feat: connect TUI to database and load tasks"
```

---

### Task 2: Render Tasks in Main Content Area

**Objective:** Display tasks in the main content area with proper formatting.

**Steps:**

1. Update `crates/todomrs-tui/src/ui.rs`:

```rust
fn draw_main_content(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let title = match app.current_view {
        View::Inbox => "Inbox",
        View::Today => "Today",
        View::Upcoming => "Upcoming",
        View::Projects => "Projects",
    };

    let filtered = app.filtered_tasks();

    if filtered.is_empty() {
        let content = Paragraph::new("No tasks. Press 'a' to add one.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(content, area);
        return;
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let priority_indicator = match task.priority {
                todomrs_core::domain::Priority::Urgent => "!!! ",
                todomrs_core::domain::Priority::High => "!! ",
                todomrs_core::domain::Priority::Medium => "! ",
                _ => "",
            };

            let status_icon = if task.status == todomrs_core::domain::TaskStatus::Completed {
                "✓ "
            } else if task.is_overdue() {
                "⚠ "
            } else {
                "□ "
            };

            let due_str = task
                .due_at
                .map(|dt| dt.format("%m/%d").to_string())
                .unwrap_or_default();

            let line = format!(
                "{}{}{} {}",
                status_icon,
                priority_indicator,
                task.title,
                if due_str.is_empty() {
                    String::new()
                } else {
                    format!("[{}]", due_str)
                }
            );

            let style = if task.status == todomrs_core::domain::TaskStatus::Completed {
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::CROSSED_OUT)
            } else if task.is_overdue() {
                Style::default().fg(Color::Red)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!("{} ({})", title, filtered.len())))
        .highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.selected_index));

    f.render_stateful_widget(list, area, &mut state);
}
```

2. Verify it compiles and runs:
```bash
cargo build --bin todomrs
cargo run --bin todomrs
```

Expected: TUI shows tasks with status icons, priority indicators, and due dates.

**Commit:**
```bash
git add .
git commit -m "feat: render tasks in main content area with formatting"
```

---

### Task 3: Add Delete Task Functionality

**Objective:** Implement task deletion with 'd' key.

**Steps:**

1. Add to `crates/todomrs-tui/src/app.rs`:

```rust
pub async fn handle_event(&mut self, event: Event) -> Result<()> {
    // ... existing code ...
    match key.code {
        // ... existing keys ...
        KeyCode::Char('d') => self.delete_task().await?,
        // ...
    }
}

async fn delete_task(&mut self) -> Result<()> {
    let filtered = self.filtered_tasks();
    if self.selected_index >= filtered.len() {
        return Ok(());
    }

    let task = filtered[self.selected_index].clone();
    self.task_store.delete(task.id).await?;
    self.refresh_tasks().await?;

    if self.selected_index > 0 {
        self.selected_index -= 1;
    }

    Ok(())
}
```

2. Update help text:
```rust
Line::from("d - Delete task"),
Line::from("x - Toggle complete"),
```

3. Verify it compiles and runs:
```bash
cargo build --bin todomrs
cargo run --bin todomrs
```

Expected: Press 'd' to delete selected task.

**Commit:**
```bash
git add .
git commit -m "feat: add delete task functionality to TUI"
```

---

## Verification

Run all checks:

```bash
cargo build --bin todomrs
cargo run --bin todomrs
```

Test the following:
1. Launch TUI
2. Press 'a' to add task: `Buy groceries tomorrow +personal p2`
3. Press Enter
4. Task appears in Inbox
5. Press '2' for Today view (task should appear if due today)
6. Press 'x' to complete task
7. Press 'd' to delete task
8. Press 'q' to quit

## Pitfalls

1. **Don't forget async/await.** All store operations are async.

2. **Don't skip error handling.** Use `?` operator and propagate errors.

3. **Don't block the UI.** Database operations should be fast.

4. **Don't forget to refresh tasks after changes.** Always call `refresh_tasks()` after create/update/delete.

## Handoff to Next Phase

Phase 7 will assume:
- TUI fully functional with task CRUD
- Natural language input working
- Views filtering tasks correctly
- All changes persisted to SQLite

Phase 7 will build the backend API for sync.
