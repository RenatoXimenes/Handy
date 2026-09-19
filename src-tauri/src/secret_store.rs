use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const SERVICE: &str = "com.pais.handy.transcription";

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
        || std::env::var("HANDY_SECRET_BACKEND")
            .map(|value| value == "memory")
            .unwrap_or(false)
}

fn attributes_for(profile_id: &str) -> [(&'static str, String); 2] {
    [
        ("service", SERVICE.to_string()),
        ("account", format!("endpoint:{profile_id}")),
    ]
}

pub async fn set_secret_async(profile_id: &str, secret: &str) -> Result<(), String> {
    if profile_id.trim().is_empty() {
        return Err("Endpoint id is required".to_string());
    }
    if secret.is_empty() {
        return delete_secret_async(profile_id).await;
    }

    if use_memory_backend() {
        memory_store()
            .lock()
            .map_err(|_| "Secret store is unavailable".to_string())?
            .insert(profile_id.to_string(), secret.to_string());
        return Ok(());
    }

    let keyring = oo7::Keyring::new()
        .await
        .map_err(|e| format!("Failed to open the system keyring: {e}"))?;
    let _ = keyring.unlock().await;
    keyring
        .create_item(
            &format!("Handy transcription ({profile_id})"),
            &attributes_for(profile_id),
            secret.as_bytes(),
            true,
        )
        .await
        .map_err(|e| format!("Failed to store secret in the system keyring: {e}"))?;
    secret_cache()
        .lock()
        .map_err(|_| "Secret cache is unavailable".to_string())?
        .insert(profile_id.to_string(), secret.to_string());
    Ok(())
}

pub async fn get_secret_async(profile_id: &str) -> Result<Option<String>, String> {
    if profile_id.trim().is_empty() {
        return Ok(None);
    }

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

    let keyring = oo7::Keyring::new()
        .await
        .map_err(|e| format!("Failed to open the system keyring: {e}"))?;
    let items = keyring
        .search_items(&attributes_for(profile_id))
        .await
        .map_err(|e| format!("Failed to read secret from the system keyring: {e}"))?;
    let Some(item) = items.into_iter().next() else {
        return Ok(None);
    };
    let secret = item
        .secret()
        .await
        .map_err(|e| format!("Failed to read secret from the system keyring: {e}"))?;
    let text = String::from_utf8_lossy(secret.as_bytes()).into_owned();
    if text.is_empty() {
        Ok(None)
    } else {
        secret_cache()
            .lock()
            .map_err(|_| "Secret cache is unavailable".to_string())?
            .insert(profile_id.to_string(), text.clone());
        Ok(Some(text))
    }
}

pub async fn delete_secret_async(profile_id: &str) -> Result<(), String> {
    if profile_id.trim().is_empty() {
        return Ok(());
    }

    if use_memory_backend() {
        memory_store()
            .lock()
            .map_err(|_| "Secret store is unavailable".to_string())?
            .remove(profile_id);
        return Ok(());
    }

    let keyring = oo7::Keyring::new()
        .await
        .map_err(|e| format!("Failed to open the system keyring: {e}"))?;
    keyring
        .delete(&attributes_for(profile_id))
        .await
        .map_err(|e| format!("Failed to remove secret from the system keyring: {e}"))?;
    secret_cache()
        .lock()
        .map_err(|_| "Secret cache is unavailable".to_string())?
        .remove(profile_id);
    Ok(())
}

pub fn set_secret(profile_id: &str, secret: &str) -> Result<(), String> {
    tauri::async_runtime::block_on(set_secret_async(profile_id, secret))
}

pub fn get_secret(profile_id: &str) -> Result<Option<String>, String> {
    tauri::async_runtime::block_on(get_secret_async(profile_id))
}

pub fn delete_secret(profile_id: &str) -> Result<(), String> {
    tauri::async_runtime::block_on(delete_secret_async(profile_id))
}

pub fn has_secret(profile_id: &str) -> bool {
    matches!(get_secret(profile_id), Ok(Some(secret)) if !secret.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_deletes_secrets_in_memory() {
        let id = "test-profile-secret-store";
        let _ = delete_secret(id);
        assert!(!has_secret(id));
        set_secret(id, "super-secret").unwrap();
        assert!(has_secret(id));
        assert_eq!(get_secret(id).unwrap().as_deref(), Some("super-secret"));
        delete_secret(id).unwrap();
        assert!(!has_secret(id));
    }
}
