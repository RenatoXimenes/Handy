use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const SERVICE: &str = "io.github.renatoximenes.vozel.transcription";
const LEGACY_SERVICE: &str = "com.pais.handy.transcription";
const MAX_PROFILE_ID_LEN: usize = 128;

fn memory_store() -> &'static Mutex<HashMap<String, String>> {
    static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn secret_cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn use_memory_backend() -> bool {
    cfg!(test)
        || (cfg!(debug_assertions)
            && std::env::var("HANDY_SECRET_BACKEND")
                .map(|value| value == "memory")
                .unwrap_or(false))
}

fn validate_profile_id(profile_id: &str) -> Result<(), String> {
    if profile_id.is_empty()
        || profile_id.len() > MAX_PROFILE_ID_LEN
        || !profile_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    {
        return Err("Endpoint id is invalid".to_string());
    }
    Ok(())
}

fn keyring_entry(profile_id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, &format!("endpoint:{profile_id}"))
        .map_err(|error| format!("Failed to open the system credential store: {error}"))
}

/// Copy credentials from the previous app identity once. Existing Vozel keys
/// win and the original keyring entries are preserved for the upstream app.
pub fn migrate_legacy_keys(
    data_dir: &std::path::Path,
    profile_ids: &[String],
) -> Result<(), String> {
    if use_memory_backend() {
        return Ok(());
    }
    let marker = data_dir.join("legacy_credentials_imported_v1");
    if marker.exists() {
        return Ok(());
    }
    for profile_id in profile_ids {
        validate_profile_id(profile_id)?;
        let current = keyring_entry(profile_id)?;
        match current.get_password() {
            Ok(_) => continue,
            Err(keyring::Error::NoEntry) => {}
            Err(error) => return Err(format!("Failed to inspect current credential: {error}")),
        }
        let legacy = keyring::Entry::new(LEGACY_SERVICE, &format!("endpoint:{profile_id}"))
            .map_err(|error| format!("Failed to open legacy credential: {error}"))?;
        match legacy.get_password() {
            Ok(secret) if !secret.is_empty() => current
                .set_password(&secret)
                .map_err(|error| format!("Failed to import credential: {error}"))?,
            Ok(_) | Err(keyring::Error::NoEntry) => {}
            Err(error) => return Err(format!("Failed to read legacy credential: {error}")),
        }
    }
    std::fs::write(marker, b"imported\n")
        .map_err(|error| format!("Failed to mark credential import complete: {error}"))
}

fn set_secret_blocking(profile_id: &str, secret: &str) -> Result<(), String> {
    validate_profile_id(profile_id)?;
    if secret.is_empty() {
        return delete_secret_blocking(profile_id);
    }

    if use_memory_backend() {
        memory_store()
            .lock()
            .map_err(|_| "Secret store is unavailable".to_string())?
            .insert(profile_id.to_string(), secret.to_string());
        return Ok(());
    }

    keyring_entry(profile_id)?
        .set_password(secret)
        .map_err(|error| {
            format!("Failed to store secret in the system credential store: {error}")
        })?;
    secret_cache()
        .lock()
        .map_err(|_| "Secret cache is unavailable".to_string())?
        .insert(profile_id.to_string(), secret.to_string());
    Ok(())
}

fn get_secret_blocking(profile_id: &str) -> Result<Option<String>, String> {
    validate_profile_id(profile_id)?;

    if use_memory_backend() {
        return Ok(memory_store()
            .lock()
            .map_err(|_| "Secret store is unavailable".to_string())?
            .get(profile_id)
            .cloned());
    }

    if let Some(secret) = secret_cache()
        .lock()
        .map_err(|_| "Secret cache is unavailable".to_string())?
        .get(profile_id)
        .cloned()
    {
        return Ok(Some(secret));
    }

    let secret = match keyring_entry(profile_id)?.get_password() {
        Ok(secret) => secret,
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Failed to read secret from the system credential store: {error}"
            ))
        }
    };
    if secret.is_empty() {
        return Ok(None);
    }
    secret_cache()
        .lock()
        .map_err(|_| "Secret cache is unavailable".to_string())?
        .insert(profile_id.to_string(), secret.clone());
    Ok(Some(secret))
}

fn delete_secret_blocking(profile_id: &str) -> Result<(), String> {
    validate_profile_id(profile_id)?;

    if use_memory_backend() {
        memory_store()
            .lock()
            .map_err(|_| "Secret store is unavailable".to_string())?
            .remove(profile_id);
        return Ok(());
    }

    match keyring_entry(profile_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {}
        Err(error) => {
            return Err(format!(
                "Failed to remove secret from the system credential store: {error}"
            ))
        }
    }
    secret_cache()
        .lock()
        .map_err(|_| "Secret cache is unavailable".to_string())?
        .remove(profile_id);
    Ok(())
}

async fn run_blocking<T, F>(operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| format!("Credential store worker failed: {error}"))?
}

pub async fn set_secret(profile_id: &str, secret: &str) -> Result<(), String> {
    let profile_id = profile_id.to_string();
    let secret = secret.to_string();
    run_blocking(move || set_secret_blocking(&profile_id, &secret)).await
}

pub async fn get_secret(profile_id: &str) -> Result<Option<String>, String> {
    let profile_id = profile_id.to_string();
    run_blocking(move || get_secret_blocking(&profile_id)).await
}

pub async fn delete_secret(profile_id: &str) -> Result<(), String> {
    let profile_id = profile_id.to_string();
    run_blocking(move || delete_secret_blocking(&profile_id)).await
}

pub async fn has_secret(profile_id: &str) -> Result<bool, String> {
    Ok(get_secret(profile_id)
        .await?
        .is_some_and(|secret| !secret.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stores_and_deletes_secrets_in_memory() {
        let id = "test-profile-secret-store";
        let _ = delete_secret(id).await;
        assert!(!has_secret(id).await.unwrap());
        set_secret(id, "super-secret").await.unwrap();
        assert!(has_secret(id).await.unwrap());
        assert_eq!(
            get_secret(id).await.unwrap().as_deref(),
            Some("super-secret")
        );
        delete_secret(id).await.unwrap();
        assert!(!has_secret(id).await.unwrap());
    }

    #[tokio::test]
    async fn rejects_invalid_profile_ids() {
        assert!(set_secret("../invalid", "secret").await.is_err());
    }
}
