/// Account-scoped storage helpers for TodoRS TUI.
///
/// Provides path computation, device/offline identity management,
/// and per-account database URL generation.
///
/// Per-account database isolation is the primary fix for cross-account
/// sync corruption. Each authenticated Supabase user gets their own
/// local SQLite database file.
use anyhow::{Context, Result};
use std::path::PathBuf;
use uuid::Uuid;

// ── Path helpers ────────────────────────────────────────────────────────────

/// Return the config directory: `~/.config/todomrs`.
pub fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("todomrs")
}

/// Return the data directory: `~/.local/share/todomrs`.
pub fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("todomrs")
}

/// Return the path to the global device ID file:
/// `~/.config/todomrs/device_id`.
pub fn device_id_path() -> PathBuf {
    config_dir().join("device_id")
}

/// Return the path to the offline fallback user ID file:
/// `~/.config/todomrs/offline_user_id`.
pub fn offline_user_id_path() -> PathBuf {
    config_dir().join("offline_user_id")
}

/// Return the path to the legacy shared database: `./todomrs.db`.
pub fn legacy_db_path() -> PathBuf {
    PathBuf::from("./todomrs.db")
}

/// Return the path to the per-account database for the given account key.
///
/// The account key is the Supabase user UUID for synced accounts, or the
/// literal string `"offline-local"` for no-auth mode.
///
/// Result: `~/.local/share/todomrs/accounts/<account-key>/todomrs.db`
pub fn database_path_for_user(account_key: &str) -> PathBuf {
    data_dir()
        .join("accounts")
        .join(account_key)
        .join("todomrs.db")
}

/// Convert a filesystem path to a `sqlite://` URL with `mode=rwc`.
pub fn sqlite_url_for_path(path: &PathBuf) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
    format!("sqlite://{}?mode=rwc", canonical.display())
}

/// Return the path to the legacy `.todomrs_user_id` file.
pub fn legacy_user_id_path() -> PathBuf {
    PathBuf::from("./.todomrs_user_id")
}

/// Return the path to the legacy `.todomrs_device_id` file.
pub fn legacy_device_id_path() -> PathBuf {
    PathBuf::from("./.todomrs_device_id")
}

// ── UUID helpers ────────────────────────────────────────────────────────────

/// Load a UUID from a file, or create and persist a new one.
///
/// Creates parent directories if needed. Uses 0o600 permissions on Unix.
pub fn load_or_create_uuid(path: &PathBuf) -> Uuid {
    let _ = ensure_parent_dir(path);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| Uuid::parse_str(s.trim()).ok())
        .unwrap_or_else(|| {
            let id = Uuid::new_v4();
            let _ = std::fs::write(path, id.to_string());
            // Restrict permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(path) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o600);
                    let _ = std::fs::set_permissions(path, perms);
                }
            }
            id
        })
}

/// Ensure the parent directory of a path exists, creating it if needed.
pub fn ensure_parent_dir(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir() {
        let dir = config_dir();
        assert!(dir.to_string_lossy().contains(".config/todomrs"));
    }

    #[test]
    fn test_data_dir() {
        let dir = data_dir();
        assert!(dir.to_string_lossy().contains(".local/share/todomrs"));
    }

    #[test]
    fn test_device_id_path() {
        let path = device_id_path();
        assert!(path.to_string_lossy().contains(".config/todomrs/device_id"));
    }

    #[test]
    fn test_offline_user_id_path() {
        let path = offline_user_id_path();
        assert!(path
            .to_string_lossy()
            .contains(".config/todomrs/offline_user_id"));
    }

    #[test]
    fn test_legacy_db_path() {
        let path = legacy_db_path();
        assert_eq!(path, PathBuf::from("./todomrs.db"));
    }

    #[test]
    fn test_database_path_for_user() {
        let path = database_path_for_user("test-user-123");
        let s = path.to_string_lossy();
        assert!(s.contains(".local/share/todomrs/accounts/test-user-123/todomrs.db"));
    }

    #[test]
    fn test_database_path_for_offline() {
        let path = database_path_for_user("offline-local");
        let s = path.to_string_lossy();
        assert!(s.contains("offline-local/todomrs.db"));
    }

    #[test]
    fn test_sqlite_url_for_path() {
        let path = PathBuf::from("/tmp/test.db");
        let url = sqlite_url_for_path(&path);
        // Should contain sqlite:// prefix and mode=rwc
        assert!(url.starts_with("sqlite://"));
        assert!(url.contains("mode=rwc"));
        assert!(url.contains("test.db"));
    }

    #[test]
    fn test_legacy_user_id_path() {
        let path = legacy_user_id_path();
        assert_eq!(path, PathBuf::from("./.todomrs_user_id"));
    }

    #[test]
    fn test_legacy_device_id_path() {
        let path = legacy_device_id_path();
        assert_eq!(path, PathBuf::from("./.todomrs_device_id"));
    }

    #[test]
    fn test_no_cwd_dependence() {
        // config_dir and data_dir use HOME, not cwd
        let config = config_dir();
        let data = data_dir();
        assert!(!config.to_string_lossy().starts_with("./"));
        assert!(!data.to_string_lossy().starts_with("./"));
    }

    #[test]
    fn test_offline_account_key_literal() {
        let path = database_path_for_user("offline-local");
        assert!(path.to_string_lossy().contains("offline-local"));
    }

    #[test]
    fn test_load_or_create_uuid_creates_and_loads() {
        let tmp = std::env::temp_dir().join(format!("test_uuid_{}", Uuid::new_v4()));
        // First call should create
        let id1 = load_or_create_uuid(&tmp);
        // Second call should load
        let id2 = load_or_create_uuid(&tmp);
        assert_eq!(id1, id2, "UUID should be stable across loads");
        // Clean up
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_ensure_parent_dir_creates() {
        let tmp = std::env::temp_dir().join(format!("test_parent_{}/nested/file.txt", Uuid::new_v4()));
        assert!(ensure_parent_dir(&tmp).is_ok());
        assert!(tmp.parent().unwrap().exists());
        // Clean up
        let _ = std::fs::remove_dir_all(tmp.parent().unwrap());
    }
}
