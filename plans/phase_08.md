# Phase 8: TUI Sync Client Integration

## Session Goal

Integrate the sync client into the TUI. Implement automatic sync on startup, manual sync with 'S' key, and operation upload/download. By the end of this session, the TUI should sync tasks between devices via Supabase.

## Expected Outcome

- TUI authenticates with Supabase on startup
- Operations are uploaded to backend after local changes
- Remote operations are downloaded and applied
- Manual sync with 'S' key
- Automatic sync on startup
- Sync status indicator in status bar
- Conflict handling (basic last-write-wins)
- `cargo run --bin todomrs` syncs with backend

## Context

Phase 7 is complete. You have:
- Supabase backend with operations table
- Edge functions for upload/download
- Rust sync client library
- Authentication configured

Now you'll integrate sync into the TUI. Every local change creates an operation. On sync, operations are uploaded and remote operations are downloaded and applied.

## Prerequisites

- Phase 7 complete and committed
- Supabase backend working
- Sync client library ready
- Test user created in Supabase

## Tasks

### Task 1: Add Sync Configuration and Authentication

**Objective:** Add configuration for Supabase credentials and authenticate on startup.

**Steps:**

1. Create `crates/todomrs-tui/src/config.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub supabase_url: String,
    pub supabase_api_key: String,
    pub email: String,
    pub password: String,
    pub user_id: Option<String>,
    pub device_id: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            let config: Config = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            // Create default config
            let config = Config {
                supabase_url: "https://YOUR_PROJECT.supabase.co".to_string(),
                supabase_api_key: "YOUR_ANON_KEY".to_string(),
                email: "test@example.com".to_string(),
                password: "password123".to_string(),
                user_id: None,
                device_id: uuid::Uuid::new_v4().to_string(),
            };
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    fn config_path() -> anyhow::Result<PathBuf> {
        let home = std::env::var("HOME")?;
        let config_dir = PathBuf::from(home).join(".config").join("todomrs");
        fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("config.json"))
    }
}
```

2. Update `crates/todomrs-tui/src/main.rs`:

```rust
mod app;
mod ui;
mod config;

use anyhow::Result;
use app::App;
use config::Config;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use todomrs_store::{Database, TaskStore};
use todomrs_sync::SyncClient;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = Config::load()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup database
    let db = Database::new("sqlite:./todomrs.db").await?;
    let task_store = TaskStore::new(db.pool().clone());

    // Setup sync client
    let mut sync_client = SyncClient::new(config.supabase_url.clone(), config.supabase_api_key.clone());
    
    // Authenticate
    let _ = sync_client.login(&config.email, &config.password).await;

    // Create app
    let user_id = config.user_id
        .and_then(|id| Uuid::parse_str(&id).ok())
        .unwrap_or_else(Uuid::new_v4);
    let device_id = Uuid::parse_str(&config.device_id).unwrap_or_else(|_| Uuid::new_v4());

    // Ensure user exists
    sqlx::query("INSERT OR IGNORE INTO users (id, email) VALUES (?, ?)")
        .bind(user_id.to_string())
        .bind(&config.email)
        .execute(db.pool())
        .await?;

    let mut app = App::new(user_id, device_id, task_store, sync_client);
    app.refresh_tasks().await?;
    
    // Initial sync
    if let Err(e) = app.sync().await {
        eprintln!("Initial sync failed: {}", e);
    }

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

3. Verify it compiles:
```bash
cargo build --bin todomrs
```

Expected: Compiles successfully.

**Commit:**
```bash
git add .
git commit -m "feat: add sync configuration and authentication to TUI"
```

---

### Task 2: Implement Sync Logic

**Objective:** Implement upload and download of operations.

**Steps:**

1. Update `crates/todomrs-tui/src/app.rs`:

```rust
use todomrs_sync::SyncClient;
use todomrs_store::OperationStore;

pub struct App {
    // ... existing fields ...
    pub sync_client: SyncClient,
    pub operation_store: OperationStore,
    pub last_synced_seq: i64,
    pub sync_status: String,
}

impl App {
    pub fn new(
        user_id: Uuid,
        device_id: Uuid,
        task_store: TaskStore,
        sync_client: SyncClient,
    ) -> Self {
        let operation_store = OperationStore::new(task_store.pool().clone());
        Self {
            // ... initialize existing fields ...
            sync_client,
            operation_store,
            last_synced_seq: 0,
            sync_status: "Not synced".to_string(),
        }
    }

    pub async fn sync(&mut self) -> Result<()> {
        self.sync_status = "Syncing...".to_string();

        // Upload local operations
        let unsynced = self.operation_store.get_unsynced(self.user_id).await?;
        if !unsynced.is_empty() {
            self.sync_client.upload_operations(unsynced.clone()).await?;
            
            let op_ids: Vec<Uuid> = unsynced.iter().map(|op| op.op_id).collect();
            self.operation_store.mark_synced(&op_ids).await?;
        }

        // Download remote operations
        let remote_ops = self.sync_client.get_operations(self.last_synced_seq).await?;
        for op in remote_ops {
            self.apply_remote_operation(op).await?;
            if op.seq > self.last_synced_seq {
                self.last_synced_seq = op.seq;
            }
        }

        self.refresh_tasks().await?;
        self.sync_status = "Synced".to_string();

        Ok(())
    }

    async fn apply_remote_operation(&mut self, op: todomrs_sync::Operation) -> Result<()> {
        use todomrs_sync::operations::{Entity, OperationPayload, OperationType};

        match (&op.entity, &op.op_type) {
            (Entity::Task, OperationType::Create) => {
                if let OperationPayload::TaskCreate { title, .. } = &op.payload {
                    let mut task = todomrs_core::domain::Task::new(self.user_id, title.clone());
                    task.id = op.entity_id;
                    // Apply other fields from payload
                    self.task_store.create(&task).await?;
                }
            }
            (Entity::Task, OperationType::Update) => {
                if let Some(mut task) = self.task_store.get_by_id(op.entity_id).await? {
                    if let OperationPayload::TaskUpdate { title, status, .. } = &op.payload {
                        if let Some(title) = title {
                            task.title = title.clone();
                        }
                        if let Some(status) = status {
                            task.status = status.clone();
                        }
                    }
                    self.task_store.update(&task).await?;
                }
            }
            (Entity::Task, OperationType::Delete) => {
                self.task_store.delete(op.entity_id).await?;
            }
            _ => {}
        }

        Ok(())
    }

    pub async fn handle_event(&mut self, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            match self.input_mode {
                InputMode::Normal => {
                    match key.code {
                        // ... existing keys ...
                        KeyCode::Char('S') => {
                            self.sync().await?;
                        }
                        // ...
                    }
                }
                // ...
            }
        }
        Ok(())
    }
}
```

2. Update task creation to also create operation:

```rust
async fn create_task_from_input(&mut self) -> Result<()> {
    if self.input_buffer.trim().is_empty() {
        return Ok(());
    }

    let (task, _recurrence) =
        NaturalLanguageParser::create_task_from_input(&self.input_buffer, self.user_id);

    self.task_store.create(&task).await?;

    // Create operation
    let seq = self.operation_store.get_next_seq(self.user_id, self.device_id).await?;
    let op = todomrs_sync::Operation::create_task(self.user_id, self.device_id, seq, &task);
    self.operation_store.append(&op).await?;

    self.refresh_tasks().await?;

    Ok(())
}
```

3. Update task completion to create operation:

```rust
async fn toggle_complete(&mut self) -> Result<()> {
    let filtered = self.filtered_tasks();
    if self.selected_index >= filtered.len() {
        return Ok(());
    }

    let mut task = filtered[self.selected_index].clone();
    let was_completed = task.status == todomrs_core::domain::TaskStatus::Completed;

    if was_completed {
        task.status = todomrs_core::domain::TaskStatus::Pending;
        task.completed_at = None;
    } else {
        task.complete();
    }
    task.updated_at = chrono::Utc::now();

    self.task_store.update(&task).await?;

    // Create operation
    let seq = self.operation_store.get_next_seq(self.user_id, self.device_id).await?;
    let op = if was_completed {
        todomrs_sync::Operation::update_task_title(self.user_id, self.device_id, seq, task.id, task.title.clone())
    } else {
        todomrs_sync::Operation::complete_task(self.user_id, self.device_id, seq, task.id)
    };
    self.operation_store.append(&op).await?;

    self.refresh_tasks().await?;

    Ok(())
}
```

4. Verify it compiles:
```bash
cargo build --bin todomrs
```

Expected: Compiles successfully.

**Commit:**
```bash
git add .
git commit -m "feat: implement sync logic for operation upload/download"
```

---

### Task 3: Add Sync Status Indicator

**Objective:** Show sync status in the status bar.

**Steps:**

1. Update `crates/todomrs-tui/src/ui.rs`:

```rust
fn draw_status_bar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let view_name = match app.current_view {
        View::Inbox => "Inbox",
        View::Today => "Today",
        View::Upcoming => "Upcoming",
        View::Projects => "Projects",
    };

    let sync_color = if app.sync_status == "Synced" {
        Color::Green
    } else if app.sync_status == "Syncing..." {
        Color::Yellow
    } else {
        Color::Red
    };

    let status = Line::from(vec![
        Span::styled(" TodoRS ", Style::default().bg(Color::Blue).fg(Color::White)),
        Span::raw(format!(" {} ", view_name)),
        Span::raw("| "),
        Span::styled("S", Style::default().fg(Color::Yellow)),
        Span::raw(" Sync "),
        Span::styled(&app.sync_status, Style::default().fg(sync_color)),
        Span::raw(" | "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit "),
        Span::styled("?", Style::default().fg(Color::Yellow)),
        Span::raw(" Help"),
    ]);

    let paragraph = Paragraph::new(status).style(Style::default().bg(Color::DarkGray));
    f.render_widget(paragraph, area);
}
```

2. Update help text:
```rust
Line::from("S - Sync now"),
```

3. Verify it compiles and runs:
```bash
cargo build --bin todomrs
cargo run --bin todomrs
```

Expected: Status bar shows sync status. Press 'S' to sync manually.

**Commit:**
```bash
git add .
git commit -m "feat: add sync status indicator to TUI"
```

---

## Verification

Test the full sync flow:

1. Launch TUI on device 1
2. Add a task: `Test sync task +test`
3. Press 'S' to sync
4. Launch TUI on device 2 (or same machine with different device_id)
5. Press 'S' to sync
6. Task should appear on device 2

## Pitfalls

1. **Don't skip error handling.** Sync can fail. Show errors to user.

2. **Don't forget to create operations for all changes.** Every local change must create an operation.

3. **Don't ignore remote operations.** Apply them to local database.

4. **Don't sync too frequently.** Rate limit sync to avoid overwhelming backend.

## Handoff to Next Phase

Phase 9 will assume:
- TUI sync working
- Operations uploaded/downloaded
- Multi-device sync functional

Phase 9 will build the PWA mobile client.
