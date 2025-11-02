pub mod hints;
pub mod retry;

use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::{header::HeaderMap, Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

use self::hints::{
    parse_gemini_error_hints, parse_retry_after_header, GeminiErrorHints, RetryHint,
};

static PLACEHOLDER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\{\d+\}|%\d*\$?s|%\d*\$?d|\$\{[^}]+\}|\\n|\\r|\\t)")
        .expect("valid placeholder regex")
});

#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("{provider} request rate limited: {message}")]
    RateLimited {
        provider: ProviderId,
        message: String,
    },
    #[error("{provider} transient network error: {message}")]
    NetworkTransient {
        provider: ProviderId,
        message: String,
    },
    #[error("{provider} transient server error ({status:?}): {message}")]
    ServerTransient {
        provider: ProviderId,
        status: Option<StatusCode>,
        message: String,
    },
    #[error("{provider} unauthorized: {message}")]
    Unauthorized {
        provider: ProviderId,
        message: String,
    },
    #[error("{provider} forbidden: {message}")]
    Forbidden {
        provider: ProviderId,
        message: String,
    },
    #[error("{provider} model unavailable: {model_id} ({message})")]
    ModelNotFound {
        provider: ProviderId,
        model_id: String,
        message: String,
        status: Option<StatusCode>,
        retry_hint: Option<RetryHint>,
    },
    #[error("placeholder mismatch: {0:?}")]
    PlaceholderMismatch(Vec<String>),
    #[error("{provider} io error: {message}")]
    IoError {
        provider: ProviderId,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Gemini,
    Gpt,
    Claude,
    Grok,
}

impl ProviderId {
    pub fn label(&self) -> &'static str {
        match self {
            ProviderId::Gemini => "Gemini",
            ProviderId::Gpt => "GPT",
            ProviderId::Claude => "Claude",
            ProviderId::Grok => "Grok",
        }
    }
}

impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl TryFrom<&str> for ProviderId {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "gemini" => Ok(ProviderId::Gemini),
            "gpt" => Ok(ProviderId::Gpt),
            "claude" => Ok(ProviderId::Claude),
            "grok" => Ok(ProviderId::Grok),
            _ => Err(()),
        }
    }
}

impl TranslationError {
    pub fn retry_hint(&self) -> Option<&RetryHint> {
        match self {
            TranslationError::ModelNotFound { retry_hint, .. } => retry_hint.as_ref(),
            _ => None,
        }
    }

    pub fn status_code(&self) -> Option<StatusCode> {
        match self {
            TranslationError::ServerTransient { status, .. } => *status,
            TranslationError::ModelNotFound { status, .. } => *status,
            _ => None,
        }
    }
}

fn map_translation_http_error(
    provider: ProviderId,
    model_id: &str,
    status: StatusCode,
    headers: HeaderMap,
    body: String,
) -> TranslationError {
    let gemini_hints = if matches!(provider, ProviderId::Gemini) {
        parse_gemini_error_hints(&body)
    } else {
        GeminiErrorHints::default()
    };

    let mut retry_hint = parse_retry_after_header(&headers);
    if retry_hint.is_none() {
        if let ProviderId::Gemini = provider {
            if let Some(hint) = gemini_hints.retry_hint.clone() {
                retry_hint = Some(hint);
            }
        }
    }

    let message = if body.trim().is_empty() {
        status.to_string()
    } else {
        body
    };
    let lowered = message.to_ascii_lowercase();

    if status == StatusCode::TOO_MANY_REQUESTS || lowered.contains("rate limit") {
        return TranslationError::RateLimited { provider, message };
    }

    if status == StatusCode::UNAUTHORIZED {
        return TranslationError::Unauthorized { provider, message };
    }

    if status == StatusCode::FORBIDDEN || gemini_hints.quota_failure {
        return TranslationError::Forbidden { provider, message };
    }

    if status == StatusCode::NOT_FOUND {
        return TranslationError::ModelNotFound {
            provider,
            model_id: model_id.to_string(),
            message,
            status: Some(status),
            retry_hint,
        };
    }

    if lowered.contains("insufficient_quota") || lowered.contains("plan required") {
        return TranslationError::Forbidden { provider, message };
    }

    if status.is_server_error() || status == StatusCode::REQUEST_TIMEOUT {
        return TranslationError::ServerTransient {
            provider,
            status: Some(status),
            message,
        };
    }

    if status.is_client_error() {
        return TranslationError::Forbidden { provider, message };
    }

    TranslationError::ServerTransient {
        provider,
        status: Some(status),
        message,
    }
}

pub async fn translate_text(
    client: &Client,
    provider: ProviderId,
    api_key: &str,
    model_id: &str,
    input: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<String, TranslationError> {
    let normalized_input = input.trim();
    if normalized_input.is_empty() {
        return Ok(String::new());
    }

    let translated = match provider {
        ProviderId::Gemini => {
            translate_with_gemini(
                client,
                api_key,
                model_id,
                normalized_input,
                source_lang,
                target_lang,
            )
            .await?
        }
        ProviderId::Gpt => {
            translate_with_gpt(
                client,
                api_key,
                model_id,
                normalized_input,
                source_lang,
                target_lang,
            )
            .await?
        }
        ProviderId::Claude => {
            translate_with_claude(
                client,
                api_key,
                model_id,
                normalized_input,
                source_lang,
                target_lang,
            )
            .await?
        }
        ProviderId::Grok => {
            translate_with_grok(
                client,
                api_key,
                model_id,
                normalized_input,
                source_lang,
                target_lang,
            )
            .await?
        }
    };

    ensure_placeholder_integrity(normalized_input, &translated)?;
    Ok(translated.trim().to_string())
}

async fn translate_with_gemini(
    client: &Client,
    api_key: &str,
    model_id: &str,
    input: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<String, TranslationError> {
    let prompt = format!(
        "Translate the following text from {source_lang} to {target_lang}. \
Preserve any placeholders such as {{0}}, %1$s, or similar tokens exactly as they appear.\n\n{input}"
    );

    let trimmed_model = model_id.trim();
    if trimmed_model.is_empty() {
        return Err(TranslationError::Forbidden {
            provider: ProviderId::Gemini,
            message: "Gemini 모델이 지정되지 않았습니다.".into(),
        });
    }
    let normalized_model = if trimmed_model.starts_with("models/") {
        trimmed_model.to_string()
    } else {
        format!("models/{trimmed_model}")
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/{normalized_model}:generateContent?key={api_key}"
    );
    let response = client
        .post(url)
        .json(&serde_json::json!({
            "contents": [{ "parts": [{ "text": prompt }] }]
        }))
        .send()
        .await
        .map_err(|err| TranslationError::NetworkTransient {
            provider: ProviderId::Gemini,
            message: err.to_string(),
        })?;

    let status = response.status();
    let headers = response.headers().clone();
    let body_bytes = response
        .bytes()
        .await
        .map_err(|err| TranslationError::NetworkTransient {
            provider: ProviderId::Gemini,
            message: err.to_string(),
        })?;

    if !status.is_success() {
        let body = String::from_utf8_lossy(&body_bytes).into_owned();
        return Err(map_translation_http_error(
            ProviderId::Gemini,
            trimmed_model,
            status,
            headers,
            body,
        ));
    }

    let parsed: GeminiResponse =
        serde_json::from_slice(&body_bytes).map_err(|err| TranslationError::ServerTransient {
            provider: ProviderId::Gemini,
            status: Some(status),
            message: err.to_string(),
        })?;
    let text = parsed
        .candidates
        .and_then(|candidates| candidates.into_iter().next())
        .and_then(|candidate| candidate.content)
        .and_then(|content| content.parts.and_then(|parts| parts.into_iter().next()))
        .and_then(|part| part.text)
        .ok_or_else(|| TranslationError::ServerTransient {
            provider: ProviderId::Gemini,
            status: Some(status),
            message: "Gemini 응답에서 결과를 찾지 못했습니다.".into(),
        })?;

    Ok(text)
}

async fn translate_with_gpt(
    client: &Client,
    api_key: &str,
    model_id: &str,
    input: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<String, TranslationError> {
    let prompt = format!(
        "Translate the following text from {source_lang} to {target_lang}. Preserve all placeholders such as {{0}} or %1$s exactly as they appear. Return only the translated text.\n\n{input}"
    );

    let trimmed_model = model_id.trim();
    if trimmed_model.is_empty() {
        return Err(TranslationError::Forbidden {
            provider: ProviderId::Gpt,
            message: "OpenAI 모델이 지정되지 않았습니다.".into(),
        });
    }

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": trimmed_model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a professional game localization translator. Preserve formatting and placeholders exactly."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.2
        }))
        .send()
        .await
        .map_err(|err| TranslationError::NetworkTransient {
            provider: ProviderId::Gpt,
            message: err.to_string(),
        })?;

    let status = response.status();
    let headers = response.headers().clone();
    let body_bytes = response
        .bytes()
        .await
        .map_err(|err| TranslationError::NetworkTransient {
            provider: ProviderId::Gpt,
            message: err.to_string(),
        })?;

    if !status.is_success() {
        let body = String::from_utf8_lossy(&body_bytes).into_owned();
        return Err(map_translation_http_error(
            ProviderId::Gpt,
            trimmed_model,
            status,
            headers,
            body,
        ));
    }

    let parsed: OpenAiResponse =
        serde_json::from_slice(&body_bytes).map_err(|err| TranslationError::ServerTransient {
            provider: ProviderId::Gpt,
            status: Some(status),
            message: err.to_string(),
        })?;
    let text = parsed
        .choices
        .into_iter()
        .find_map(|choice| choice.message.and_then(|message| message.content))
        .ok_or_else(|| TranslationError::ServerTransient {
            provider: ProviderId::Gpt,
            status: Some(status),
            message: "GPT 응답에서 결과를 찾지 못했습니다.".into(),
        })?;

    Ok(text)
}

async fn translate_with_claude(
    client: &Client,
    api_key: &str,
    model_id: &str,
    input: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<String, TranslationError> {
    let prompt = format!(
        "Translate the following text from {source_lang} to {target_lang}. Preserve all placeholders exactly as they appear, including tokens like {{0}} or %1$s. Return only the translated text.\n\n{input}"
    );

    let trimmed_model = model_id.trim();
    if trimmed_model.is_empty() {
        return Err(TranslationError::Forbidden {
            provider: ProviderId::Claude,
            message: "Claude 모델이 지정되지 않았습니다.".into(),
        });
    }

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": trimmed_model,
            "max_tokens": 1024,
            "system": "You are a professional game localization translator. Preserve formatting and placeholders exactly.",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.2
        }))
        .send()
        .await
        .map_err(|err| TranslationError::NetworkTransient {
            provider: ProviderId::Claude,
            message: err.to_string(),
        })?;

    let status = response.status();
    let headers = response.headers().clone();
    let body_bytes = response
        .bytes()
        .await
        .map_err(|err| TranslationError::NetworkTransient {
            provider: ProviderId::Claude,
            message: err.to_string(),
        })?;

    if !status.is_success() {
        let body = String::from_utf8_lossy(&body_bytes).into_owned();
        return Err(map_translation_http_error(
            ProviderId::Claude,
            trimmed_model,
            status,
            headers,
            body,
        ));
    }

    let parsed: AnthropicResponse =
        serde_json::from_slice(&body_bytes).map_err(|err| TranslationError::ServerTransient {
            provider: ProviderId::Claude,
            status: Some(status),
            message: err.to_string(),
        })?;
    let text = parsed
        .content
        .unwrap_or_default()
        .into_iter()
        .find_map(|block| block.text)
        .ok_or_else(|| TranslationError::ServerTransient {
            provider: ProviderId::Claude,
            status: Some(status),
            message: "Claude 응답에서 결과를 찾지 못했습니다.".into(),
        })?;

    Ok(text)
}

async fn translate_with_grok(
    client: &Client,
    api_key: &str,
    model_id: &str,
    input: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<String, TranslationError> {
    let prompt = format!(
        "Translate the following text from {source_lang} to {target_lang}. Preserve all placeholders such as {{0}} or %1$s exactly as they appear. Return only the translated text.\n\n{input}"
    );

    let trimmed_model = model_id.trim();
    if trimmed_model.is_empty() {
        return Err(TranslationError::Forbidden {
            provider: ProviderId::Grok,
            message: "Grok 모델이 지정되지 않았습니다.".into(),
        });
    }

    let response = client
        .post("https://api.x.ai/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": trimmed_model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a professional game localization translator. Preserve formatting and placeholders exactly."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.2
        }))
        .send()
        .await
        .map_err(|err| TranslationError::NetworkTransient {
            provider: ProviderId::Grok,
            message: err.to_string(),
        })?;

    let status = response.status();
    let headers = response.headers().clone();
    let body_bytes = response
        .bytes()
        .await
        .map_err(|err| TranslationError::NetworkTransient {
            provider: ProviderId::Grok,
            message: err.to_string(),
        })?;

    if !status.is_success() {
        let body = String::from_utf8_lossy(&body_bytes).into_owned();
        return Err(map_translation_http_error(
            ProviderId::Grok,
            trimmed_model,
            status,
            headers,
            body,
        ));
    }

    let parsed: OpenAiResponse =
        serde_json::from_slice(&body_bytes).map_err(|err| TranslationError::ServerTransient {
            provider: ProviderId::Grok,
            status: Some(status),
            message: err.to_string(),
        })?;
    let text = parsed
        .choices
        .into_iter()
        .find_map(|choice| choice.message.and_then(|message| message.content))
        .ok_or_else(|| TranslationError::ServerTransient {
            provider: ProviderId::Grok,
            status: Some(status),
            message: "Grok 응답에서 결과를 찾지 못했습니다.".into(),
        })?;

    Ok(text)
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Option<Vec<GeminiPart>>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: Option<OpenAiMessage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Option<Vec<AnthropicContentBlock>>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(default)]
    text: Option<String>,
}

fn collect_placeholders(input: &str) -> Vec<String> {
    PLACEHOLDER_REGEX
        .captures_iter(input)
        .map(|caps| caps[0].to_string())
        .collect()
}

fn ensure_placeholder_integrity(original: &str, translated: &str) -> Result<(), TranslationError> {
    let expected = collect_placeholders(original);
    let actual = collect_placeholders(translated);

    let expected_map = expected.iter().fold(BTreeMap::new(), |mut acc, token| {
        *acc.entry(token).or_insert(0_usize) += 1;
        acc
    });
    let actual_map = actual.iter().fold(BTreeMap::new(), |mut acc, token| {
        *acc.entry(token).or_insert(0_usize) += 1;
        acc
    });

    if expected_map != actual_map {
        let missing: Vec<String> = expected_map
            .iter()
            .filter_map(|(token, expected_count)| {
                let actual_count = actual_map.get(*token).copied().unwrap_or_default();
                if actual_count < *expected_count {
                    Some(format!(
                        "{token} (missing {})",
                        expected_count - actual_count
                    ))
                } else {
                    None
                }
            })
            .collect();

        if !missing.is_empty() {
            return Err(TranslationError::PlaceholderMismatch(missing));
        }
    }

    Ok(())
}
