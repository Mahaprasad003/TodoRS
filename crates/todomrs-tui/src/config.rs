use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for the TodoRS TUI.
///
/// Stored at `~/.config/todomrs/config.json`.
/// Created automatically on first run with placeholder values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub supabase_url: String,
    pub supabase_api_key: String,
    pub email: String,
    pub password: String,
}

impl Config {
    /// Load config from `~/.config/todomrs/config.json`.
    ///
    /// Creates the file with placeholder values if it doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config {
                supabase_url: "https://YOUR_PROJECT.supabase.co".to_string(),
                supabase_api_key: "YOUR_ANON_KEY".to_string(),
                email: String::new(),
                password: String::new(),
            };
            config.save()?;
            eprintln!("Config created at: {}", path.display());
            eprintln!("Edit it with your Supabase credentials and restart.");
            Ok(config)
        }
    }

    /// Save the config to disk with restricted permissions.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, &content)?;

        // Restrict permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&path) {
                let mut perms = meta.permissions();
                perms.set_mode(0o600);
                let _ = std::fs::set_permissions(&path, perms);
            }
        }

        Ok(())
    }

    /// Returns the config file path (`~/.config/todomrs/config.json`).
    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".config")
            .join("todomrs")
            .join("config.json")
    }

    /// Returns `true` if the config has real credentials (not placeholders).
    pub fn is_configured(&self) -> bool {
        !self.supabase_url.contains("YOUR_PROJECT")
            && !self.supabase_api_key.contains("YOUR_ANON_KEY")
            && !self.email.is_empty()
            && !self.password.is_empty()
    }
}
