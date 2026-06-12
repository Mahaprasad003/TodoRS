mod app;
mod config;
mod notifications;
mod storage;
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
use std::io::{self, Write};
use todomrs_store::{Database, OperationStore, ProjectStore, RecurrenceRuleStore, TaskStore};

/// Prompt the user for a line of input, showing a label.
fn prompt(label: &str) -> String {
    print!("{}: ", label);
    let _ = io::stdout().flush();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap_or_default();
    input.trim().to_string()
}

/// Handle the `todomrs login` CLI command.
async fn cmd_login() -> Result<()> {
    let config = Config::load()?;

    if config.supabase_url.contains("YOUR_PROJECT") || config.supabase_api_key.contains("YOUR_ANON_KEY") {
        eprintln!("Supabase is not configured.");
        eprintln!("First edit: {}", Config::config_path().display());
        return Ok(());
    }

    let mut client = todomrs_sync::SyncClient::new(
        config.supabase_url.clone(),
        config.supabase_api_key.clone(),
    );

    println!("Enter your TodoRS account credentials.");
    loop {
        let email = prompt("Email");
        if email.is_empty() {
            eprintln!("Email cannot be empty.");
            continue;
        }

        let password = prompt("Password");
        if password.is_empty() {
            eprintln!("Password cannot be empty.");
            continue;
        }

        match client.login(&email, &password).await {
            Ok(token) if !token.is_empty() => {
                let mut new_config = config.clone();
                new_config.email = email;
                new_config.password = password;
                new_config.save()?;
                println!("Login successful. Credentials saved to config.");
                println!("To switch accounts, run: todomrs login again to overwrite credentials.");
        println!("Your local task data is now automatically scoped by account.");
                return Ok(());
            }
            Ok(_) => {
                eprintln!("Login returned an empty token. Please try again.");
            }
            Err(e) => {
                eprintln!("Login failed: {}. Please try again.", e);
            }
        }
    }
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
    let args: Vec<String> = std::env::args().collect();

    // `todomrs login` — CLI login flow, no TUI needed
    if args.get(1).map(|s| s.as_str()) == Some("login") {
        return cmd_login().await;
    }

    // Normal TUI startup
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
    // ── Compute effective account identity ────────────────────────────
    //
    // In synced mode, the authenticated Supabase user ID is the canonical
    // local user_id. In offline/no-auth mode, we use a stable local UUID.
    //
    // The account_key determines which per-account database file is used.
    //
    // Guard: if the sync client exists but supabase_user_id() is None,
    // we disable sync to prevent identity mismatch between local user_id
    // and the JWT's auth user ID.
    let sync_client = sync_client.and_then(|client| {
        if client.supabase_user_id().is_some() {
            Some(client)
        } else {
            eprintln!("Warning: Sync login succeeded but no user ID was returned.");
            eprintln!("  Falling back to offline mode. Sync is disabled.");
            None
        }
    });

    let (effective_user_id, account_key, effective_email) = if let Some(ref client) = sync_client {
        let supabase_id = client.supabase_user_id()
            .expect("just checked is_some above");
        (supabase_id, supabase_id.to_string(), String::new())
    } else {
        let uid = storage::load_or_create_uuid(&storage::offline_user_id_path());
        (uid, "offline-local".to_string(), "local@todomrs".to_string())
    };

    // ── Emit warning about legacy shared state ────────────────────────
    // Task 3: Quarantine legacy shared local state — detect and warn
    let legacy_db = storage::legacy_db_path();
    let legacy_uid = storage::legacy_user_id_path();
    let legacy_did = storage::legacy_device_id_path();
    let has_legacy = legacy_db.exists() || legacy_uid.exists() || legacy_did.exists();
    if has_legacy {
        eprintln!("Warning: Legacy shared local state detected (./todomrs.db, ./.todomrs_*).");
        eprintln!("  This build uses per-account storage at: {}",
            storage::database_path_for_user(&account_key).display());
        eprintln!("  Legacy files are left untouched and will not be used.");
    }

    // ── Open per-account database ─────────────────────────────────────
    let db_path = storage::database_path_for_user(&account_key);
    storage::ensure_parent_dir(&db_path)
        .expect("Failed to create database parent directory");
    let db_url = storage::sqlite_url_for_path(&db_path);
    let db = Database::new(&db_url).await?;
    sqlx::migrate!("../../migrations").run(db.pool()).await?;

    let task_store = TaskStore::new(db.pool().clone());
    let op_store = OperationStore::new(db.pool().clone());
    let project_store = ProjectStore::new(db.pool().clone());
    let recurrence_store = RecurrenceRuleStore::new(db.pool().clone());

    // ── Global device identity (per installation, not per account) ────
    let device_id = storage::load_or_create_uuid(&storage::device_id_path());

    // ── Ensure local users row matches the effective user ─────────────
    // In synced mode, the canonical local user_id equals the Supabase auth user ID.
    // In offline mode, it's a stable local UUID stored in offline_user_id_path().
    sqlx::query("INSERT OR IGNORE INTO users (id, email) VALUES (?, ?)")
        .bind(effective_user_id)
        .bind(effective_email)
        .execute(db.pool())
        .await?;

    // In synced mode, the Supabase auth is the source of truth for identity.
    // The users row already exists with the correct user_id from INSERT above.
    // The email can be left as empty; the canonical email is in Supabase auth.

    let mut app = App::new(
        effective_user_id,
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

    // Load persisted sync state (last_synced_at) so we don't redownload everything
    app.load_sync_state().await;

    app.refresh_tasks().await?;

    // Initial sync
    if app.sync_client.is_some() {
        app.sync().await?;
    }

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        // Periodic auto-sync every 30s
        app.maybe_auto_sync().await;

        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;
            app.handle_event(event).await?;
        }

        if app.should_quit {
            break;
        }
    }

    // Exit sync: upload any remaining unsynced operations before quitting
    if app.sync_client.is_some() {
        app.sync().await.ok();
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
