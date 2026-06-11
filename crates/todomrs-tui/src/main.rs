mod app;
mod config;
mod ui;

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
use todomrs_store::{Database, OperationStore, ProjectStore, RecurrenceRuleStore, TaskStore};
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

/// Initialize the sync client from config (called before raw mode so errors are visible).
async fn init_sync_client(config: &Config) -> Option<todomrs_sync::SyncClient> {
    if !config.is_configured() {
        eprintln!(
            "Sync not configured. Edit: {}",
            Config::config_path().display()
        );
        return None;
    }

    let mut client = todomrs_sync::SyncClient::new(
        config.supabase_url.clone(),
        config.supabase_api_key.clone(),
    );

    match client.login(&config.email, &config.password).await {
        Ok(token) if !token.is_empty() => {
            eprintln!("Sync login successful");
            Some(client)
        }
        Ok(_) => {
            eprintln!("Sync login returned empty token");
            None
        }
        Err(e) => {
            eprintln!("Sync login failed: {}", e);
            None
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Install panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = restore_terminal();
        original_hook(panic);
    }));

    // Load config and init sync client BEFORE raw mode (so errors are visible)
    let config = Config::load()?;
    let sync_client = init_sync_client(&config).await;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the async TUI loop
    let result = run_async(&mut terminal, sync_client).await;

    // Always restore terminal
    let _ = terminal.show_cursor();
    restore_terminal()?;
    result
}

async fn run_async(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    sync_client: Option<todomrs_sync::SyncClient>,
) -> Result<()> {
    let db = Database::new("sqlite://./todomrs.db?mode=rwc").await?;
    sqlx::migrate!("../../migrations").run(db.pool()).await?;

    let task_store = TaskStore::new(db.pool().clone());
    let op_store = OperationStore::new(db.pool().clone());
    let project_store = ProjectStore::new(db.pool().clone());
    let recurrence_store = RecurrenceRuleStore::new(db.pool().clone());

    // Load or create persistent device identity
    let user_id = load_or_create_id(".todomrs_user_id");
    let device_id = load_or_create_id(".todomrs_device_id");

    // Ensure local user exists (bind Uuid directly — stored as BLOB, matching sqlx's Uuid type)
    sqlx::query("INSERT OR IGNORE INTO users (id, email) VALUES (?, ?)")
        .bind(user_id)
        .bind("local@todomrs")
        .execute(db.pool())
        .await?;

    let mut app = App::new(
        user_id,
        device_id,
        task_store,
        op_store,
        project_store,
        recurrence_store,
    );

    // Inject sync client if available
    if let Some(client) = sync_client {
        app.set_sync_client(client);
    }

    app.refresh_tasks().await?;

    // Initial sync
    if app.sync_client.is_some() {
        app.sync().await?;
    }

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
