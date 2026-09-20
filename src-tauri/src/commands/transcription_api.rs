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

fn view_of(
    profile: &TranscriptionEndpointProfile,
    active_id: &str,
    has_secret: bool,
) -> TranscriptionEndpointView {
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
        has_secret,
    }
}

fn validate_input(input: &TranscriptionEndpointInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("Endpoint name is required".to_string());
    }
    if input.base_url.trim().is_empty() {
        return Err("Base URL is required".to_string());
    }
    if input.model.trim().is_empty() {
        return Err("Model is required".to_string());
    }
    if !(5..=300).contains(&input.timeout_secs) {
        return Err("Timeout must be between 5 and 300 seconds".to_string());
    }
    asr_client::endpoint_url(&input.base_url, &input.transcription_path)
        .map_err(|error| error.user_message())?;
    if !input.extra_params_json.trim().is_empty() {
        let value: serde_json::Value = serde_json::from_str(&input.extra_params_json)
            .map_err(|_| "Extra JSON parameters must be a valid JSON object".to_string())?;
        if !value.is_object() {
            return Err("Extra JSON parameters must be a JSON object".to_string());
        }
        if value.as_object().is_some_and(|object| {
            object
                .keys()
                .any(|key| matches!(key.as_str(), "file" | "model" | "language"))
        }) {
            return Err(
                "Extra JSON parameters cannot override file, model, or language".to_string(),
            );
        }
        if value.as_object().is_some_and(|object| {
            object.values().any(|value| {
                !matches!(
                    value,
                    serde_json::Value::String(_)
                        | serde_json::Value::Number(_)
                        | serde_json::Value::Bool(_)
                )
            })
        }) {
            return Err(
                "Extra JSON parameter values must be strings, numbers, or booleans".to_string(),
            );
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
    format!("custom-{}", uuid::Uuid::new_v4())
}

#[tauri::command]
#[specta::specta]
pub async fn list_transcription_endpoints(
    app: AppHandle,
) -> Result<Vec<TranscriptionEndpointView>, String> {
    let settings = get_settings(&app);
    let mut views = Vec::with_capacity(settings.transcription_endpoints.len());
    for profile in &settings.transcription_endpoints {
        let has_secret = if matches!(profile.auth_type, TranscriptionAuthType::None) {
            false
        } else {
            secret_store::has_secret(&profile.id).await?
        };
        views.push(view_of(
            profile,
            &settings.active_transcription_endpoint_id,
            has_secret,
        ));
    }
    Ok(views)
}

#[tauri::command]
#[specta::specta]
pub async fn save_transcription_endpoint(
    app: AppHandle,
    input: TranscriptionEndpointInput,
    secret: Option<String>,
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
    let security_settings_changed = settings
        .transcription_endpoint(&id)
        .filter(|profile| !profile.is_preset)
        .is_some_and(|profile| {
            profile.base_url != input.base_url.trim()
                || profile.auth_type != input.auth_type
                || profile.auth_header_name.as_deref().unwrap_or("")
                    != input
                        .auth_header_name
                        .as_deref()
                        .map(str::trim)
                        .unwrap_or("")
        });

    if let Some(existing) = settings.transcription_endpoint_mut(&id) {
        if !existing.is_preset {
            existing.name = input.name.trim().to_string();
            existing.base_url = input.base_url.trim().to_string();
            existing.transcription_path = if input.transcription_path.trim().is_empty() {
                "/audio/transcriptions".to_string()
            } else {
                input.transcription_path.trim().to_string()
            };
            existing.auth_type = input.auth_type;
            existing.auth_header_name = input
                .auth_header_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string());
        }
        existing.model = input.model.trim().to_string();
        existing.send_language = input.send_language;
        existing.extra_params_json = input.extra_params_json.trim().to_string();
        existing.timeout_secs = input.timeout_secs.clamp(5, 300);
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
                timeout_secs: input.timeout_secs.clamp(5, 300),
                is_preset: false,
            });
    }

    let saved_profile = settings
        .transcription_endpoint(&id)
        .cloned()
        .ok_or_else(|| "Failed to save transcription endpoint".to_string())?;
    if matches!(saved_profile.auth_type, TranscriptionAuthType::None) {
        secret_store::delete_secret(&id).await?;
    } else if let Some(secret) = secret
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        secret_store::set_secret(&id, &secret).await?;
    } else if security_settings_changed {
        // Never reuse a credential silently after the destination or auth scheme changes.
        secret_store::delete_secret(&id).await?;
    }
    write_settings(&app, settings);
    let has_secret = if matches!(saved_profile.auth_type, TranscriptionAuthType::None) {
        false
    } else {
        secret_store::has_secret(&id).await?
    };
    Ok(view_of(
        &saved_profile,
        &get_settings(&app).active_transcription_endpoint_id,
        has_secret,
    ))
}

#[tauri::command]
#[specta::specta]
pub async fn delete_transcription_endpoint(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    let profile = settings
        .transcription_endpoint(&id)
        .cloned()
        .ok_or_else(|| "Endpoint not found".to_string())?;
    if profile.is_preset {
        return Err("Preset endpoints cannot be deleted".to_string());
    }
    secret_store::delete_secret(&id).await?;
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
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn duplicate_transcription_endpoint(
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
    let copied_secret = if matches!(source.auth_type, TranscriptionAuthType::None) {
        None
    } else {
        secret_store::get_secret(&id).await?
    };
    if let Some(secret) = copied_secret.as_deref() {
        secret_store::set_secret(&new_id, secret).await?;
    }
    settings.transcription_endpoints.push(copy.clone());
    write_settings(&app, settings);
    Ok(view_of(
        &copy,
        &get_settings(&app).active_transcription_endpoint_id,
        copied_secret.is_some(),
    ))
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
pub async fn set_transcription_endpoint_secret(
    app: AppHandle,
    id: String,
    secret: String,
) -> Result<(), String> {
    let settings = get_settings(&app);
    let profile = settings
        .transcription_endpoint(&id)
        .ok_or_else(|| "Endpoint not found".to_string())?;
    if matches!(profile.auth_type, TranscriptionAuthType::None) {
        return Err("This endpoint does not use authentication".to_string());
    }
    secret_store::set_secret(&id, secret.trim()).await
}

#[tauri::command]
#[specta::specta]
pub async fn clear_transcription_endpoint_secret(app: AppHandle, id: String) -> Result<(), String> {
    if get_settings(&app).transcription_endpoint(&id).is_none() {
        return Err("Endpoint not found".to_string());
    }
    secret_store::delete_secret(&id).await
}

#[tauri::command]
#[specta::specta]
pub async fn has_transcription_endpoint_secret(app: AppHandle, id: String) -> Result<bool, String> {
    let settings = get_settings(&app);
    let profile = settings
        .transcription_endpoint(&id)
        .ok_or_else(|| "Endpoint not found".to_string())?;
    if matches!(profile.auth_type, TranscriptionAuthType::None) {
        return Ok(false);
    }
    secret_store::has_secret(&id).await
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
    let api_key = if matches!(profile.auth_type, TranscriptionAuthType::None) {
        None
    } else {
        secret_store::get_secret(&id).await?
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_input() -> TranscriptionEndpointInput {
        TranscriptionEndpointInput {
            id: None,
            name: "Example".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            transcription_path: "/audio/transcriptions".to_string(),
            model: "whisper-model".to_string(),
            auth_type: TranscriptionAuthType::Bearer,
            auth_header_name: None,
            send_language: true,
            extra_params_json: String::new(),
            timeout_secs: 60,
        }
    }

    #[test]
    fn accepts_a_valid_remote_endpoint() {
        assert!(validate_input(&valid_input()).is_ok());
    }

    #[test]
    fn rejects_insecure_remote_http_and_hidden_destinations() {
        let mut input = valid_input();
        input.base_url = "http://api.example.com/v1".to_string();
        assert!(validate_input(&input).is_err());

        input.base_url = "https://api.example.com/v1".to_string();
        input.transcription_path = "https://other.example.com/capture".to_string();
        assert!(validate_input(&input).is_err());
    }

    #[test]
    fn rejects_reserved_or_structured_extra_parameters() {
        let mut input = valid_input();
        input.extra_params_json = r#"{"model":"override"}"#.to_string();
        assert!(validate_input(&input).is_err());

        input.extra_params_json = r#"{"metadata":{"private":true}}"#.to_string();
        assert!(validate_input(&input).is_err());
    }
}
