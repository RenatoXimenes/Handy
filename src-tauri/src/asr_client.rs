use crate::settings::{TranscriptionAuthType, TranscriptionEndpointProfile};
use hound::{SampleFormat, WavSpec, WavWriter};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use serde::Deserialize;
use serde_json::Value;
use std::error::Error as StdError;
use std::io::Cursor;
use std::sync::OnceLock;
use std::time::Duration;

pub const API_TRANSCRIPTION_MODEL_ID: &str = "api-transcription";
const MAX_ATTEMPTS: u32 = 3;

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsrError {
    InvalidKey,
    RateLimited,
    Unavailable,
    Network,
    Timeout,
    MissingKey,
    MissingEndpoint,
    Other(String),
}

impl AsrError {
    pub fn user_message(&self) -> String {
        match self {
            AsrError::InvalidKey => {
                "Invalid API key. Check the key stored in the system keyring.".to_string()
            }
            AsrError::RateLimited => {
                "The transcription API rate limit was exceeded. Try again in a moment.".to_string()
            }
            AsrError::Unavailable => {
                "The transcription API is temporarily unavailable.".to_string()
            }
            AsrError::Network => {
                "Could not reach the transcription API. Check your internet connection.".to_string()
            }
            AsrError::Timeout => "The transcription API timed out.".to_string(),
            AsrError::MissingKey => {
                "No API key is stored for the active endpoint. Save a key first.".to_string()
            }
            AsrError::MissingEndpoint => "No transcription API endpoint is selected.".to_string(),
            AsrError::Other(message) => message.clone(),
        }
    }
}

impl std::fmt::Display for AsrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl std::error::Error for AsrError {}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: Option<ApiErrorMessage>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorMessage {
    message: Option<String>,
}

pub fn whisper_language_code(language: &str) -> Option<String> {
    let trimmed = language.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return None;
    }
    let base = trimmed.split(['-', '_']).next().unwrap_or(trimmed);
    if base.is_empty() {
        None
    } else {
        Some(base.to_ascii_lowercase())
    }
}

pub fn join_endpoint_url(base_url: &str, path: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    let path = path.trim();
    if path.is_empty() {
        format!("{base}/audio/transcriptions")
    } else if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else if path.starts_with('/') {
        format!("{base}{path}")
    } else {
        format!("{base}/{path}")
    }
}

pub fn encode_wav_16k_mono(samples: &[f32]) -> Result<Vec<u8>, AsrError> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec).map_err(|e| {
            AsrError::Other(format!(
                "Failed to encode WAV for the transcription API: {e}"
            ))
        })?;
        for sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let pcm = (clamped * i16::MAX as f32) as i16;
            writer
                .write_sample(pcm)
                .map_err(|e| AsrError::Other(format!("Failed to write WAV sample: {e}")))?;
        }
        writer
            .finalize()
            .map_err(|e| AsrError::Other(format!("Failed to finalize WAV payload: {e}")))?;
    }
    Ok(cursor.into_inner())
}

pub fn silent_wav(duration_secs: f32) -> Result<Vec<u8>, AsrError> {
    let frames = (16_000.0 * duration_secs.max(0.05)) as usize;
    encode_wav_16k_mono(&vec![0.0; frames])
}

fn build_headers(
    profile: &TranscriptionEndpointProfile,
    api_key: Option<&str>,
) -> Result<HeaderMap, AsrError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        HeaderValue::from_static("Handy/0.9.5 (+https://github.com/cjpais/Handy)"),
    );

    match profile.auth_type {
        TranscriptionAuthType::None => {}
        TranscriptionAuthType::Bearer => {
            let key = api_key
                .filter(|value| !value.is_empty())
                .ok_or(AsrError::MissingKey)?;
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {key}")).map_err(|_| {
                    AsrError::Other(
                        "The stored API key contains invalid header characters.".to_string(),
                    )
                })?,
            );
        }
        TranscriptionAuthType::CustomHeader => {
            let key = api_key
                .filter(|value| !value.is_empty())
                .ok_or(AsrError::MissingKey)?;
            let header_name = profile
                .auth_header_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AsrError::Other("Custom authentication header name is required.".to_string())
                })?;
            let name = HeaderName::from_bytes(header_name.as_bytes()).map_err(|_| {
                AsrError::Other("Custom authentication header name is invalid.".to_string())
            })?;
            headers.insert(
                name,
                HeaderValue::from_str(key).map_err(|_| {
                    AsrError::Other(
                        "The stored API key contains invalid header characters.".to_string(),
                    )
                })?,
            );
        }
    }

    Ok(headers)
}

fn extra_form_fields(extra_params_json: &str) -> Result<Vec<(String, String)>, AsrError> {
    let trimmed = extra_params_json.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = serde_json::from_str(trimmed).map_err(|_| {
        AsrError::Other("Extra JSON parameters must be a valid JSON object.".to_string())
    })?;
    let object = value.as_object().ok_or_else(|| {
        AsrError::Other("Extra JSON parameters must be a JSON object.".to_string())
    })?;
    Ok(object
        .iter()
        .map(|(key, value)| {
            let serialized = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            (key.clone(), serialized)
        })
        .collect())
}

fn classify_status(status: u16, body: &str) -> AsrError {
    let api_message = serde_json::from_str::<ApiErrorBody>(body)
        .ok()
        .and_then(|parsed| {
            parsed
                .error
                .and_then(|error| error.message)
                .or(parsed.message)
        });

    match status {
        401 | 403 => AsrError::InvalidKey,
        429 => AsrError::RateLimited,
        500..=599 => AsrError::Unavailable,
        _ => AsrError::Other(
            api_message.unwrap_or_else(|| format!("Transcription API returned HTTP {status}")),
        ),
    }
}

fn classify_reqwest(error: &reqwest::Error) -> AsrError {
    if error.is_timeout() {
        return AsrError::Timeout;
    }
    if error.is_connect() {
        return AsrError::Network;
    }
    if error.is_request() || error.is_body() {
        let mut source = error.source();
        while let Some(cause) = source {
            let text = cause.to_string().to_ascii_lowercase();
            if text.contains("timed out") || text.contains("timeout") {
                return AsrError::Timeout;
            }
            if text.contains("connection") || text.contains("dns") || text.contains("network") {
                return AsrError::Network;
            }
            source = cause.source();
        }
        return AsrError::Network;
    }
    AsrError::Other("Transcription API request failed".to_string())
}

fn is_retryable(error: &AsrError) -> bool {
    matches!(
        error,
        AsrError::RateLimited | AsrError::Unavailable | AsrError::Network | AsrError::Timeout
    )
}

pub async fn transcribe_wav(
    profile: &TranscriptionEndpointProfile,
    api_key: Option<&str>,
    wav: Vec<u8>,
    language: Option<&str>,
) -> Result<String, AsrError> {
    let url = join_endpoint_url(&profile.base_url, &profile.transcription_path);
    let extra_fields = extra_form_fields(&profile.extra_params_json)?;
    let timeout = Duration::from_secs(profile.timeout_secs.max(5));
    let headers = build_headers(profile, api_key)?;
    let client = http_client();

    let mut last_error = AsrError::Unavailable;
    for attempt in 0..MAX_ATTEMPTS {
        let mut form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(wav.clone())
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .map_err(|e| AsrError::Other(format!("Failed to attach WAV payload: {e}")))?,
        );
        if !profile.model.trim().is_empty() {
            form = form.text("model", profile.model.clone());
        }
        if let Some(code) = language
            .filter(|_| profile.send_language)
            .and_then(whisper_language_code)
        {
            form = form.text("language", code);
        }
        for (key, value) in &extra_fields {
            form = form.text(key.clone(), value.clone());
        }

        let response = match client
            .post(&url)
            .headers(headers.clone())
            .timeout(timeout)
            .multipart(form)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                last_error = classify_reqwest(&error);
                if is_retryable(&last_error) && attempt + 1 < MAX_ATTEMPTS {
                    std::thread::sleep(Duration::from_millis(400 * (attempt as u64 + 1)));
                    continue;
                }
                return Err(last_error);
            }
        };

        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|e| AsrError::Other(format!("Failed to read API response: {e}")))?;

        if (200..300).contains(&status) {
            if let Ok(parsed) = serde_json::from_str::<TranscriptionResponse>(&body) {
                return Ok(parsed.text.unwrap_or_default());
            }
            return Ok(body.trim().trim_matches('"').to_string());
        }

        last_error = classify_status(status, &body);
        if is_retryable(&last_error) && attempt + 1 < MAX_ATTEMPTS {
            std::thread::sleep(Duration::from_millis(400 * (attempt as u64 + 1)));
            continue;
        }
        return Err(last_error);
    }

    Err(last_error)
}

pub async fn transcribe_samples(
    profile: &TranscriptionEndpointProfile,
    api_key: Option<&str>,
    samples: &[f32],
    language: Option<&str>,
) -> Result<String, AsrError> {
    let wav = encode_wav_16k_mono(samples)?;
    transcribe_wav(profile, api_key, wav, language).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{default_groq_transcription_endpoint, TranscriptionAuthType};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn maps_pt_br_to_whisper_pt() {
        assert_eq!(whisper_language_code("pt-BR").as_deref(), Some("pt"));
        assert_eq!(whisper_language_code("pt_BR").as_deref(), Some("pt"));
        assert_eq!(whisper_language_code("auto"), None);
        assert_eq!(whisper_language_code("").as_deref(), None);
    }

    #[test]
    fn joins_openai_compatible_urls() {
        assert_eq!(
            join_endpoint_url("https://api.groq.com/openai/v1", "/audio/transcriptions"),
            "https://api.groq.com/openai/v1/audio/transcriptions"
        );
        assert_eq!(
            join_endpoint_url("http://localhost:8000/v1/", "audio/transcriptions"),
            "http://localhost:8000/v1/audio/transcriptions"
        );
    }

    #[test]
    fn encodes_valid_wav() {
        let wav = encode_wav_16k_mono(&[0.1, -0.2, 0.0]).unwrap();
        assert!(wav.starts_with(b"RIFF"));
        assert!(wav.len() > 44);
    }

    fn spawn_mock<F>(expected_requests: usize, handler: F) -> (String, thread::JoinHandle<()>)
    where
        F: Fn(String) -> (u16, String) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            for _ in 0..expected_requests {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let mut buffer = vec![0u8; 16_384];
                let _ = stream.read(&mut buffer);
                let request = String::from_utf8_lossy(&buffer).to_string();
                let (status, body) = handler(request);
                let reason = match status {
                    200 => "OK",
                    401 => "Unauthorized",
                    429 => "Too Many Requests",
                    _ => "Error",
                };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (format!("http://{addr}/v1"), handle)
    }

    fn test_profile(base_url: String) -> TranscriptionEndpointProfile {
        let mut profile = default_groq_transcription_endpoint();
        profile.base_url = base_url;
        profile.timeout_secs = 5;
        profile
    }

    #[tokio::test]
    async fn transcribes_from_mock_server() {
        let (base_url, handle) = spawn_mock(1, |request| {
            assert!(request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-key"));
            assert!(request.contains("whisper-large-v3-turbo"));
            assert!(request.contains("name=\"language\""));
            assert!(request.contains("pt"));
            (200, r#"{"text":"ola mundo"}"#.to_string())
        });
        let result = transcribe_wav(
            &test_profile(base_url),
            Some("test-key"),
            silent_wav(0.1).unwrap(),
            Some("pt-BR"),
        )
        .await
        .unwrap();
        assert_eq!(result, "ola mundo");
        let _ = handle.join();
    }

    #[tokio::test]
    async fn maps_unauthorized_to_invalid_key() {
        let (base_url, handle) = spawn_mock(1, |_| (401, r#"{"error":{"message":"nope"}}"#.into()));
        let error = transcribe_wav(
            &test_profile(base_url),
            Some("bad-key"),
            silent_wav(0.1).unwrap(),
            None,
        )
        .await
        .unwrap_err();
        assert_eq!(error, AsrError::InvalidKey);
        let _ = handle.join();
    }

    #[tokio::test]
    async fn retries_rate_limit_then_succeeds() {
        let attempts = Arc::new(Mutex::new(0u32));
        let attempts_clone = attempts.clone();
        let (base_url, handle) = spawn_mock(2, move |_| {
            let mut count = attempts_clone.lock().unwrap();
            *count += 1;
            if *count == 1 {
                (429, r#"{"error":{"message":"slow down"}}"#.into())
            } else {
                (200, r#"{"text":"ok"}"#.into())
            }
        });
        let result = transcribe_wav(
            &test_profile(base_url),
            Some("test-key"),
            silent_wav(0.1).unwrap(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(result, "ok");
        assert!(*attempts.lock().unwrap() >= 2);
        let _ = handle.join();
    }

    #[tokio::test]
    async fn sends_no_auth_header_when_disabled() {
        let (base_url, handle) = spawn_mock(1, |request| {
            assert!(!request.to_ascii_lowercase().contains("authorization"));
            (200, r#"{"text":"local"}"#.into())
        });
        let mut profile = test_profile(base_url);
        profile.auth_type = TranscriptionAuthType::None;
        let result = transcribe_wav(&profile, None, silent_wav(0.1).unwrap(), None)
            .await
            .unwrap();
        assert_eq!(result, "local");
        let _ = handle.join();
    }
}
