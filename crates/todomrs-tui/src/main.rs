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
use todomrs_store::{Database, OperationStore, ProjectStore, TaskStore};
use uuid::Uuid;

/// Load a persistent UUID from a file, or create one if it doesn't exist.
fn load_or_create_id(path: &str) -> Uuid {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| Uuid::parse_str(s.trim()).ok())
        .unwrap_or_else(|| {
            let id = Uuid::new_v4();
            let _ = std::fs::write(path, id.to_string());
            id
        })
}

#[tokio::main]
async fn main() -> Result<()> {
    // Install panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = restore_terminal();
        original_hook(panic);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the async TUI loop
    let result = run_async(&mut terminal).await;

    // Always restore terminal
    let _ = terminal.show_cursor();
    restore_terminal()?;
    result
}

async fn run_async(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let db = Database::new("sqlite://./todomrs.db?mode=rwc").await?;
    sqlx::migrate!("../../migrations").run(db.pool()).await?;

    let task_store = TaskStore::new(db.pool().clone());
    let op_store = OperationStore::new(db.pool().clone());
    let project_store = ProjectStore::new(db.pool().clone());

    // Load or create persistent device identity
    let user_id = load_or_create_id(".todomrs_user_id");
    let device_id = load_or_create_id(".todomrs_device_id");

    // Ensure local user exists (bind Uuid directly — stored as BLOB, matching sqlx's Uuid type)
    sqlx::query("INSERT OR IGNORE INTO users (id, email) VALUES (?, ?)")
        .bind(user_id)
        .bind("local@todomrs")
        .execute(db.pool())
        .await?;

    let mut app = App::new(user_id, device_id, task_store, op_store, project_store);
    app.refresh_tasks().await?;

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

    db.close().await;
    Ok(())
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode().ok();
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        crossterm::cursor::Show
    )?;
    Ok(())
}
