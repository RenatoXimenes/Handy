use crate::asr_client::{self, silent_wav};
use crate::secret_store;
use crate::settings::{
    get_settings, write_settings, TranscriptionAuthType, TranscriptionEndpointProfile,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TranscriptionEndpointView {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub transcription_path: String,
    pub model: String,
    pub auth_type: TranscriptionAuthType,
    pub auth_header_name: Option<String>,
    pub send_language: bool,
    pub extra_params_json: String,
    pub timeout_secs: u64,
    pub is_preset: bool,
    pub is_active: bool,
    pub has_secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TranscriptionEndpointInput {
    pub id: Option<String>,
    pub name: String,
    pub base_url: String,
    pub transcription_path: String,
    pub model: String,
    pub auth_type: TranscriptionAuthType,
    pub auth_header_name: Option<String>,
    pub send_language: bool,
    pub extra_params_json: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TranscriptionEndpointTestResult {
    pub ok: bool,
    pub message: String,
}

fn view_of(profile: &TranscriptionEndpointProfile, active_id: &str) -> TranscriptionEndpointView {
    TranscriptionEndpointView {
        id: profile.id.clone(),
        name: profile.name.clone(),
        base_url: profile.base_url.clone(),
        transcription_path: profile.transcription_path.clone(),
        model: profile.model.clone(),
        auth_type: profile.auth_type,
        auth_header_name: profile.auth_header_name.clone(),
        send_language: profile.send_language,
        extra_params_json: profile.extra_params_json.clone(),
        timeout_secs: profile.timeout_secs,
        is_preset: profile.is_preset,
        is_active: profile.id == active_id,
        has_secret: secret_store::has_secret(&profile.id),
    }
}

fn validate_input(input: &TranscriptionEndpointInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("Endpoint name is required".to_string());
    }
    if input.base_url.trim().is_empty() {
        return Err("Base URL is required".to_string());
    }
    if !input.extra_params_json.trim().is_empty() {
        let value: serde_json::Value = serde_json::from_str(&input.extra_params_json)
            .map_err(|_| "Extra JSON parameters must be a valid JSON object".to_string())?;
        if !value.is_object() {
            return Err("Extra JSON parameters must be a JSON object".to_string());
        }
    }
    if matches!(input.auth_type, TranscriptionAuthType::CustomHeader)
        && input
            .auth_header_name
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
    {
        return Err("Custom authentication header name is required".to_string());
    }
    Ok(())
}

fn new_endpoint_id() -> String {
    format!(
        "custom-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    )
}

#[tauri::command]
#[specta::specta]
pub fn list_transcription_endpoints(
    app: AppHandle,
) -> Result<Vec<TranscriptionEndpointView>, String> {
    let settings = get_settings(&app);
    Ok(settings
        .transcription_endpoints
        .iter()
        .map(|profile| view_of(profile, &settings.active_transcription_endpoint_id))
        .collect())
}

#[tauri::command]
#[specta::specta]
pub fn save_transcription_endpoint(
    app: AppHandle,
    input: TranscriptionEndpointInput,
) -> Result<TranscriptionEndpointView, String> {
    validate_input(&input)?;
    let mut settings = get_settings(&app);
    let id = input
        .id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(new_endpoint_id);

    if let Some(existing) = settings.transcription_endpoint_mut(&id) {
        existing.name = input.name.trim().to_string();
        existing.base_url = input.base_url.trim().to_string();
        existing.transcription_path = if input.transcription_path.trim().is_empty() {
            "/audio/transcriptions".to_string()
        } else {
            input.transcription_path.trim().to_string()
        };
        existing.model = input.model.trim().to_string();
        existing.auth_type = input.auth_type;
        existing.auth_header_name = input
            .auth_header_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());
        existing.send_language = input.send_language;
        existing.extra_params_json = input.extra_params_json.trim().to_string();
        existing.timeout_secs = input.timeout_secs.max(5);
    } else {
        settings
            .transcription_endpoints
            .push(TranscriptionEndpointProfile {
                id: id.clone(),
                name: input.name.trim().to_string(),
                base_url: input.base_url.trim().to_string(),
                transcription_path: if input.transcription_path.trim().is_empty() {
                    "/audio/transcriptions".to_string()
                } else {
                    input.transcription_path.trim().to_string()
                },
                model: input.model.trim().to_string(),
                auth_type: input.auth_type,
                auth_header_name: input
                    .auth_header_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string()),
                send_language: input.send_language,
                extra_params_json: input.extra_params_json.trim().to_string(),
                timeout_secs: input.timeout_secs.max(5),
                is_preset: false,
            });
    }

    let active_id = settings.active_transcription_endpoint_id.clone();
    let view = settings
        .transcription_endpoint(&id)
        .map(|profile| view_of(profile, &active_id))
        .ok_or_else(|| "Failed to save transcription endpoint".to_string())?;
    write_settings(&app, settings);
    Ok(view)
}

#[tauri::command]
#[specta::specta]
pub fn delete_transcription_endpoint(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    let profile = settings
        .transcription_endpoint(&id)
        .cloned()
        .ok_or_else(|| "Endpoint not found".to_string())?;
    if profile.is_preset {
        return Err("Preset endpoints cannot be deleted".to_string());
    }
    settings
        .transcription_endpoints
        .retain(|endpoint| endpoint.id != id);
    if settings.active_transcription_endpoint_id == id {
        settings.active_transcription_endpoint_id = settings
            .transcription_endpoints
            .first()
            .map(|endpoint| endpoint.id.clone())
            .unwrap_or_else(|| "groq".to_string());
    }
    write_settings(&app, settings);
    secret_store::delete_secret(&id)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn duplicate_transcription_endpoint(
    app: AppHandle,
    id: String,
) -> Result<TranscriptionEndpointView, String> {
    let mut settings = get_settings(&app);
    let source = settings
        .transcription_endpoint(&id)
        .cloned()
        .ok_or_else(|| "Endpoint not found".to_string())?;
    let new_id = new_endpoint_id();
    let copy = TranscriptionEndpointProfile {
        id: new_id.clone(),
        name: format!("{} copy", source.name),
        is_preset: false,
        ..source.clone()
    };
    settings.transcription_endpoints.push(copy);
    let active_id = settings.active_transcription_endpoint_id.clone();
    let view = settings
        .transcription_endpoint(&new_id)
        .map(|profile| view_of(profile, &active_id))
        .ok_or_else(|| "Failed to duplicate endpoint".to_string())?;
    write_settings(&app, settings);
    if let Some(secret) = secret_store::get_secret(&id)? {
        secret_store::set_secret(&new_id, &secret)?;
    }
    Ok(view)
}

#[tauri::command]
#[specta::specta]
pub fn set_active_transcription_endpoint(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    if settings.transcription_endpoint(&id).is_none() {
        return Err("Endpoint not found".to_string());
    }
    settings.active_transcription_endpoint_id = id;
    write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_transcription_endpoint_secret(id: String, secret: String) -> Result<(), String> {
    secret_store::set_secret(&id, secret.trim())
}

#[tauri::command]
#[specta::specta]
pub fn clear_transcription_endpoint_secret(id: String) -> Result<(), String> {
    secret_store::delete_secret(&id)
}

#[tauri::command]
#[specta::specta]
pub fn has_transcription_endpoint_secret(id: String) -> Result<bool, String> {
    Ok(secret_store::has_secret(&id))
}

#[tauri::command]
#[specta::specta]
pub async fn test_transcription_endpoint(
    app: AppHandle,
    id: String,
) -> Result<TranscriptionEndpointTestResult, String> {
    let settings = get_settings(&app);
    let profile = settings
        .transcription_endpoint(&id)
        .cloned()
        .ok_or_else(|| "Endpoint not found".to_string())?;
    let api_key = secret_store::get_secret_async(&id).await?;
    match asr_client::transcribe_wav(
        &profile,
        api_key.as_deref(),
        silent_wav(0.2).map_err(|e| e.user_message())?,
        Some("pt"),
    )
    .await
    {
        Ok(_) => Ok(TranscriptionEndpointTestResult {
            ok: true,
            message: "Connection successful".to_string(),
        }),
        Err(error) => Ok(TranscriptionEndpointTestResult {
            ok: false,
            message: error.user_message(),
        }),
    }
}
