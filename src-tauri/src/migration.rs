//! One-time, non-destructive import from the upstream application data location.
//! The new bundle identifier must not make an existing user's data disappear.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use tauri::{AppHandle, Manager};

const LEGACY_ID: &str = "com.pais.handy";

fn copy_file_if_missing(source: &Path, destination: &Path) -> std::io::Result<()> {
    if !source.is_file() || destination.exists() {
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(())
}

fn copy_tree_if_missing(source: &Path, destination: &Path) -> std::io::Result<()> {
    if !source.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = destination.join(entry.file_name());
        // Do not follow links from another app's data directory.
        if kind.is_dir() {
            copy_tree_if_missing(&entry.path(), &target)?;
        } else if kind.is_file() {
            copy_file_if_missing(&entry.path(), &target)?;
        }
    }
    Ok(())
}

fn snapshot_history_if_missing(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_file() || destination.exists() {
        return Ok(());
    }
    let temporary = destination.with_extension("db.importing");
    if temporary.exists() {
        fs::remove_file(&temporary).map_err(|error| error.to_string())?;
    }
    let connection = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| error.to_string())?;
    connection
        .execute("VACUUM INTO ?1", [temporary.to_string_lossy().as_ref()])
        .map_err(|error| error.to_string())?;
    fs::rename(temporary, destination).map_err(|error| error.to_string())
}

fn old_sibling(new_path: &Path) -> Option<PathBuf> {
    Some(new_path.parent()?.join(LEGACY_ID))
}

pub fn import_legacy(app: &AppHandle) -> Result<Option<PathBuf>, String> {
    if crate::portable::is_portable() {
        return Ok(None);
    }
    let destination = crate::portable::app_data_dir(app).map_err(|error| error.to_string())?;
    let Some(source) = old_sibling(&destination) else {
        return Ok(None);
    };
    if !source.is_dir() || source == destination {
        return Ok(None);
    }
    fs::create_dir_all(&destination).map_err(|error| error.to_string())?;

    // Older installations may have put the store under AppConfig rather than
    // AppData. Prefer AppData when both exist.
    let old_config = app
        .path()
        .app_config_dir()
        .ok()
        .and_then(|path| old_sibling(&path));
    let store = crate::settings::SETTINGS_STORE_PATH;
    copy_file_if_missing(&source.join(store), &destination.join(store))
        .map_err(|error| error.to_string())?;
    if let Some(config) = old_config {
        copy_file_if_missing(&config.join(store), &destination.join(store))
            .map_err(|error| error.to_string())?;
    }
    copy_tree_if_missing(&source.join("models"), &destination.join("models"))
        .map_err(|error| error.to_string())?;
    copy_tree_if_missing(&source.join("recordings"), &destination.join("recordings"))
        .map_err(|error| error.to_string())?;
    snapshot_history_if_missing(&source.join("history.db"), &destination.join("history.db"))?;

    Ok(Some(destination))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_preserves_existing_files_and_skips_links() {
        let root = std::env::temp_dir().join(format!("vozel-migration-{}", uuid::Uuid::new_v4()));
        let old = root.join("old");
        let new = root.join("new");
        fs::create_dir_all(&old).unwrap();
        fs::create_dir_all(&new).unwrap();
        fs::write(old.join("model.gguf"), b"legacy").unwrap();
        fs::write(new.join("model.gguf"), b"current").unwrap();
        fs::write(old.join("other.gguf"), b"copied").unwrap();
        copy_tree_if_missing(&old, &new).unwrap();
        assert_eq!(fs::read(new.join("model.gguf")).unwrap(), b"current");
        assert_eq!(fs::read(new.join("other.gguf")).unwrap(), b"copied");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn history_import_is_consistent_with_wal() {
        let root = std::env::temp_dir().join(format!("vozel-history-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let old = root.join("history.db");
        let new = root.join("imported.db");
        let connection = Connection::open(&old).unwrap();
        connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE entries (value TEXT); INSERT INTO entries VALUES ('saved');").unwrap();
        snapshot_history_if_missing(&old, &new).unwrap();
        let copied = Connection::open(&new).unwrap();
        let value: String = copied
            .query_row("SELECT value FROM entries", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "saved");
        drop(copied);
        drop(connection);
        fs::remove_dir_all(root).unwrap();
    }
}
