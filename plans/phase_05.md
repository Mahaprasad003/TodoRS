# Phase 5: TUI Skeleton — ratatui App Structure

## Session Goal

Build the TUI application skeleton using ratatui. Create the main application loop, layout structure, basic navigation, and keybinding system. By the end of this session, you should have a working TUI that launches, displays a basic layout, and responds to keyboard input.

## Expected Outcome

- TUI application launches and runs
- Basic layout with sidebar and main content area
- Navigation between views (Inbox, Today, Upcoming, Projects)
- Keybinding system working (j/k for navigation, q to quit)
- Application state management
- Clean terminal handling (restore on exit)
- `cargo run --bin todomrs` launches the TUI

## Context

Phase 4 is complete. You have:
- Operation log system working
- Snapshot/replay mechanism in place
- All core domain types and stores

Now you'll build the user interface. This phase creates the skeleton. Phase 6 will add actual task views and editing.

## Prerequisites

- Phase 4 complete and committed
- All stores and operation log working
- ratatui and crossterm dependencies in Cargo.toml

## Tasks

### Task 1: Create Application State and Event System

**Objective:** Define the application state structure and event handling system.

**Steps:**

1. Create `crates/todomrs-tui/src/app.rs`:

```rust
use crossterm::event::{Event, KeyCode};
use todomrs_core::domain::{Project, Task};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Inbox,
    Today,
    Upcoming,
    Projects,
}

#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub current_view: View,
    pub selected_index: usize,
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub user_id: Uuid,
    pub device_id: Uuid,
}

impl App {
    pub fn new(user_id: Uuid, device_id: Uuid) -> Self {
        Self {
            should_quit: false,
            current_view: View::Inbox,
            selected_index: 0,
            tasks: Vec::new(),
            projects: Vec::new(),
            user_id,
            device_id,
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('j') | KeyCode::Down => self.next_item(),
                KeyCode::Char('k') | KeyCode::Up => self.previous_item(),
                KeyCode::Char('1') => self.current_view = View::Inbox,
                KeyCode::Char('2') => self.current_view = View::Today,
                KeyCode::Char('3') => self.current_view = View::Upcoming,
                KeyCode::Char('4') => self.current_view = View::Projects,
                _ => {}
            }
        }
    }

    pub fn next_item(&mut self) {
        if self.selected_index < self.tasks.len().saturating_sub(1) {
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

2. Create `crates/todomrs-tui/src/ui.rs`:

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, View};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(f.size());

    draw_sidebar(f, app, chunks[0]);
    draw_main_content(f, app, chunks[1]);
}

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
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(selected));

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_main_content(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let title = match app.current_view {
        View::Inbox => "Inbox",
        View::Today => "Today",
        View::Upcoming => "Upcoming",
        View::Projects => "Projects",
    };

    let content = Paragraph::new(format!("{} view - {} tasks", title, app.tasks.len()))
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(content, area);
}
```

3. Update `crates/todomrs-tui/src/main.rs`:

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
use uuid::Uuid;

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let user_id = Uuid::new_v4(); // TODO: Load from config
    let device_id = Uuid::new_v4(); // TODO: Load from config
    let mut app = App::new(user_id, device_id);

    // Main loop
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;
            app.handle_event(event);
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

    Ok(())
}
```

4. Verify it compiles and runs:
```bash
cargo build --bin todomrs
cargo run --bin todomrs
```

Expected: TUI launches with sidebar and main content area. Press 'q' to quit.

**Commit:**
```bash
git add .
git commit -m "feat: create TUI skeleton with basic layout and navigation"
```

---

### Task 2: Add Status Bar and Help

**Objective:** Add a status bar showing current view and a help overlay.

**Steps:**

1. Update `crates/todomrs-tui/src/app.rs` to add help mode:

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
}

impl App {
    pub fn new(user_id: Uuid, device_id: Uuid) -> Self {
        Self {
            should_quit: false,
            current_view: View::Inbox,
            selected_index: 0,
            tasks: Vec::new(),
            projects: Vec::new(),
            user_id,
            device_id,
            show_help: false,
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            if self.show_help {
                if let KeyCode::Char('?') | KeyCode::Esc = key.code {
                    self.show_help = false;
                }
                return;
            }

            match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('?') => self.show_help = true,
                KeyCode::Char('j') | KeyCode::Down => self.next_item(),
                KeyCode::Char('k') | KeyCode::Up => self.previous_item(),
                KeyCode::Char('1') => self.current_view = View::Inbox,
                KeyCode::Char('2') => self.current_view = View::Today,
                KeyCode::Char('3') => self.current_view = View::Upcoming,
                KeyCode::Char('4') => self.current_view = View::Projects,
                _ => {}
            }
        }
    }
}
```

2. Update `crates/todomrs-tui/src/ui.rs` to add status bar and help:

```rust
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.size());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(chunks[0]);

    draw_sidebar(f, app, main_chunks[0]);
    draw_main_content(f, app, main_chunks[1]);
    draw_status_bar(f, app, chunks[1]);

    if app.show_help {
        draw_help(f, app);
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let view_name = match app.current_view {
        View::Inbox => "Inbox",
        View::Today => "Today",
        View::Upcoming => "Upcoming",
        View::Projects => "Projects",
    };

    let status = Line::from(vec![
        Span::styled(" TodoRS ", Style::default().bg(Color::Blue).fg(Color::White)),
        Span::raw(format!(" {} ", view_name)),
        Span::raw("| "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit "),
        Span::styled("?", Style::default().fg(Color::Yellow)),
        Span::raw(" Help "),
    ]);

    let paragraph = Paragraph::new(status).style(Style::default().bg(Color::DarkGray));
    f.render_widget(paragraph, area);
}

fn draw_help(f: &mut Frame, app: &App) {
    let area = f.size();
    let help_area = ratatui::layout::Rect {
        x: area.width / 4,
        y: area.height / 4,
        width: area.width / 2,
        height: area.height / 2,
    };

    let help_text = vec![
        Line::from(Span::styled("Keyboard Shortcuts", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("q - Quit"),
        Line::from("? - Toggle help"),
        Line::from("j/↓ - Next item"),
        Line::from("k/↑ - Previous item"),
        Line::from("1 - Inbox view"),
        Line::from("2 - Today view"),
        Line::from("3 - Upcoming view"),
        Line::from("4 - Projects view"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"));

    f.render_widget(ratatui::widgets::Clear, help_area);
    f.render_widget(paragraph, help_area);
}
```

3. Verify it compiles and runs:
```bash
cargo build --bin todomrs
cargo run --bin todomrs
```

Expected: TUI launches with status bar at bottom. Press '?' to show help overlay.

**Commit:**
```bash
git add .
git commit -m "feat: add status bar and help overlay to TUI"
```

---

### Task 3: Add Input Mode for Quick Add

**Objective:** Add an input mode that allows typing tasks with natural language.

**Steps:**

1. Update `crates/todomrs-tui/src/app.rs`:

```rust
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
}

impl App {
    pub fn new(user_id: Uuid, device_id: Uuid) -> Self {
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
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            match self.input_mode {
                InputMode::Normal => {
                    if self.show_help {
                        if let KeyCode::Char('?') | KeyCode::Esc = key.code {
                            self.show_help = false;
                        }
                        return;
                    }

                    match key.code {
                        KeyCode::Char('q') => self.should_quit = true,
                        KeyCode::Char('?') => self.show_help = true,
                        KeyCode::Char('a') => self.input_mode = InputMode::Editing,
                        KeyCode::Char('j') | KeyCode::Down => self.next_item(),
                        KeyCode::Char('k') | KeyCode::Up => self.previous_item(),
                        KeyCode::Char('1') => self.current_view = View::Inbox,
                        KeyCode::Char('2') => self.current_view = View::Today,
                        KeyCode::Char('3') => self.current_view = View::Upcoming,
                        KeyCode::Char('4') => self.current_view = View::Projects,
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
                            // TODO: Parse and create task
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
    }
}
```

2. Update `crates/todomrs-tui/src/ui.rs` to show input field:

```rust
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3), // Input field
            Constraint::Length(1), // Status bar
        ])
        .split(f.size());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(chunks[0]);

    draw_sidebar(f, app, main_chunks[0]);
    draw_main_content(f, app, main_chunks[1]);
    draw_input_field(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);

    if app.show_help {
        draw_help(f, app);
    }
}

fn draw_input_field(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let title = match app.input_mode {
        InputMode::Normal => "Press 'a' to add task",
        InputMode::Editing => "Add task (Enter to save, Esc to cancel)",
    };

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(match app.input_mode {
            InputMode::Normal => Style::default().fg(Color::DarkGray),
            InputMode::Editing => Style::default().fg(Color::White),
        })
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(input, area);

    // Show cursor in editing mode
    if app.input_mode == InputMode::Editing {
        f.set_cursor(
            area.x + app.input_buffer.len() as u16 + 1,
            area.y + 1,
        );
    }
}
```

3. Update help text to include 'a' shortcut:

```rust
Line::from("a - Add task"),
```

4. Verify it compiles and runs:
```bash
cargo build --bin todomrs
cargo run --bin todomrs
```

Expected: Press 'a' to enter input mode. Type text. Press Enter to save (currently does nothing). Press Esc to cancel.

**Commit:**
```bash
git add .
git commit -m "feat: add input mode for quick add in TUI"
```

---

## Verification

Run all checks:

```bash
cargo build --bin todomrs
cargo run --bin todomrs
```

Expected:
- TUI launches successfully
- Sidebar shows views (Inbox, Today, Upcoming, Projects)
- Status bar at bottom
- Press 'q' to quit
- Press '?' for help
- Press '1-4' to switch views
- Press 'j/k' to navigate
- Press 'a' to enter input mode
- Type text, press Enter or Esc

## Pitfalls

1. **Don't forget to restore terminal on exit.** Always disable raw mode and leave alternate screen.

2. **Don't block the event loop.** Use poll() with timeout, not blocking read().

3. **Don't ignore input mode.** Keybindings should behave differently in Normal vs Editing mode.

4. **Don't skip cursor management.** Show cursor only in editing mode.

## Handoff to Next Phase

Phase 6 will assume:
- TUI skeleton working
- Layout structure in place
- Input mode functional
- Navigation working

Phase 6 will add actual task views, task editing, and integration with the store layer.
