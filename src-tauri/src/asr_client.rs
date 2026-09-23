use crate::settings::{TranscriptionAuthType, TranscriptionEndpointProfile};
use futures_util::StreamExt;
use hound::{SampleFormat, WavSpec, WavWriter};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use reqwest::redirect::Policy;
use reqwest::Url;
use serde::Deserialize;
use serde_json::Value;
use std::error::Error as StdError;
use std::io::Cursor;
use std::sync::OnceLock;
use std::time::Duration;

pub const API_TRANSCRIPTION_MODEL_ID: &str = "api-transcription";
const MAX_ATTEMPTS: u32 = 3;
const MAX_RESPONSE_BYTES: usize = 1_048_576;
const MAX_TIMEOUT_SECS: u64 = 300;

fn http_client() -> Result<&'static reqwest::Client, AsrError> {
    static CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();
    match CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|error| error.to_string())
    }) {
        Ok(client) => Ok(client),
        Err(error) => Err(AsrError::Other(format!(
            "Failed to initialize the transcription API client: {error}"
        ))),
    }
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

fn is_loopback(url: &Url) -> bool {
    url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|address| address.is_loopback())
    })
}

pub fn endpoint_url(base_url: &str, path: &str) -> Result<Url, AsrError> {
    let base = base_url.trim().trim_end_matches('/');
    let path = path.trim();
    let parsed_base = Url::parse(base)
        .map_err(|_| AsrError::Other("The transcription base URL is invalid.".to_string()))?;
    if parsed_base.username() != ""
        || parsed_base.password().is_some()
        || parsed_base.query().is_some()
        || parsed_base.fragment().is_some()
    {
        return Err(AsrError::Other(
            "The transcription base URL cannot contain credentials, a query, or a fragment."
                .to_string(),
        ));
    }
    if parsed_base.scheme() != "https"
        && !(parsed_base.scheme() == "http" && is_loopback(&parsed_base))
    {
        return Err(AsrError::Other(
            "Remote transcription endpoints must use HTTPS; HTTP is allowed only for localhost."
                .to_string(),
        ));
    }
    let normalized_path = if path.is_empty() {
        "/audio/transcriptions"
    } else {
        path
    };
    if !normalized_path.starts_with('/')
        || normalized_path.starts_with("//")
        || normalized_path.contains("://")
        || normalized_path.contains('\\')
        || normalized_path.contains('?')
        || normalized_path.contains('#')
    {
        return Err(AsrError::Other(
            "The transcription path must be an absolute path on the configured host.".to_string(),
        ));
    }
    Url::parse(&format!("{base}{normalized_path}"))
        .map_err(|_| AsrError::Other("The transcription endpoint URL is invalid.".to_string()))
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
        HeaderValue::from_static(concat!(
            "Vozel/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/RenatoXimenes/Vozel)"
        )),
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
    object
        .iter()
        .map(|(key, value)| {
            if matches!(key.as_str(), "file" | "model" | "language") {
                return Err(AsrError::Other(format!(
                    "Extra JSON parameters cannot override the reserved field '{key}'."
                )));
            }
            let serialized = match value {
                Value::String(text) => text.clone(),
                Value::Number(_) | Value::Bool(_) => value.to_string(),
                _ => {
                    return Err(AsrError::Other(format!(
                        "Extra JSON parameter '{key}' must be a string, number, or boolean."
                    )))
                }
            };
            Ok((key.clone(), serialized))
        })
        .collect()
}

fn classify_status(status: u16) -> AsrError {
    match status {
        401 | 403 => AsrError::InvalidKey,
        429 => AsrError::RateLimited,
        500..=599 => AsrError::Unavailable,
        _ => AsrError::Other(format!(
            "The transcription API rejected the request (HTTP {status})."
        )),
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
        AsrError::RateLimited | AsrError::Unavailable | AsrError::Network
    )
}

async fn read_body_limited(response: reqwest::Response) -> Result<String, AsrError> {
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| {
            AsrError::Other(format!(
                "Failed to read the transcription API response: {error}"
            ))
        })?;
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(AsrError::Other(
                "The transcription API response exceeded the 1 MiB safety limit.".to_string(),
            ));
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body)
        .map_err(|_| AsrError::Other("The transcription API returned non-UTF-8 data.".to_string()))
}

pub async fn transcribe_wav(
    profile: &TranscriptionEndpointProfile,
    api_key: Option<&str>,
    wav: Vec<u8>,
    language: Option<&str>,
) -> Result<String, AsrError> {
    let url = endpoint_url(&profile.base_url, &profile.transcription_path)?;
    let extra_fields = extra_form_fields(&profile.extra_params_json)?;
    let timeout = Duration::from_secs(profile.timeout_secs.clamp(5, MAX_TIMEOUT_SECS));
    let headers = build_headers(profile, api_key)?;
    let client = http_client()?;

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
            .post(url.clone())
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
                    tokio::time::sleep(Duration::from_millis(400 * (attempt as u64 + 1))).await;
                    continue;
                }
                return Err(last_error);
            }
        };

        let status = response.status().as_u16();
        let body = read_body_limited(response).await?;

        if (200..300).contains(&status) {
            let parsed = serde_json::from_str::<TranscriptionResponse>(&body).map_err(|_| {
                AsrError::Other(
                    "The transcription API returned an invalid JSON response.".to_string(),
                )
            })?;
            return parsed.text.ok_or_else(|| {
                AsrError::Other(
                    "The transcription API response did not contain a text field.".to_string(),
                )
            });
        }

        last_error = classify_status(status);
        if is_retryable(&last_error) && attempt + 1 < MAX_ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(400 * (attempt as u64 + 1))).await;
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
    fn validates_and_joins_openai_compatible_urls() {
        assert_eq!(
            endpoint_url("https://api.groq.com/openai/v1", "/audio/transcriptions")
                .unwrap()
                .as_str(),
            "https://api.groq.com/openai/v1/audio/transcriptions"
        );
        assert_eq!(
            endpoint_url("http://localhost:8000/v1/", "/audio/transcriptions")
                .unwrap()
                .as_str(),
            "http://localhost:8000/v1/audio/transcriptions"
        );
        assert!(endpoint_url("http://api.example.com/v1", "/audio/transcriptions").is_err());
        assert!(endpoint_url("https://api.example.com/v1", "https://evil.example/test").is_err());
        assert!(endpoint_url(
            "https://user:secret@example.com/v1",
            "/audio/transcriptions"
        )
        .is_err());
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

    #[tokio::test]
    async fn rejects_success_responses_without_the_expected_json_schema() {
        let (base_url, handle) = spawn_mock(1, |_| (200, "<html>not a transcript</html>".into()));
        let error = transcribe_wav(
            &test_profile(base_url),
            Some("test-key"),
            silent_wav(0.1).unwrap(),
            None,
        )
        .await
        .unwrap_err();
        assert!(error.user_message().contains("invalid JSON"));
        let _ = handle.join();
    }

    #[tokio::test]
    async fn does_not_follow_redirects_with_credentials() {
        let destination = TcpListener::bind("127.0.0.1:0").unwrap();
        destination.set_nonblocking(true).unwrap();
        let destination_url = format!("http://{}/capture", destination.local_addr().unwrap());
        let source = TcpListener::bind("127.0.0.1:0").unwrap();
        let source_address = source.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = source.accept().unwrap();
            let mut buffer = [0u8; 4096];
            let _ = stream.read(&mut buffer);
            let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: {destination_url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let error = transcribe_wav(
            &test_profile(format!("http://{source_address}/v1")),
            Some("redirect-test-key"),
            silent_wav(0.1).unwrap(),
            None,
        )
        .await
        .unwrap_err();
        assert!(error.user_message().contains("HTTP 302"));
        let _ = handle.join();
        thread::sleep(Duration::from_millis(50));
        assert!(matches!(
            destination.accept(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
        ));
    }

    #[tokio::test]
    async fn rejects_oversized_responses() {
        let oversized = "x".repeat(MAX_RESPONSE_BYTES + 1);
        let (base_url, handle) = spawn_mock(1, move |_| (200, oversized.clone()));
        let error = transcribe_wav(
            &test_profile(base_url),
            Some("test-key"),
            silent_wav(0.1).unwrap(),
            None,
        )
        .await
        .unwrap_err();
        assert!(error.user_message().contains("1 MiB"));
        let _ = handle.join();
    }
}
