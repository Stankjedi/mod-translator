use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::{header::HeaderMap, Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

static PLACEHOLDER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\{\d+\}|%\d*\$?s|%\d*\$?d|\$\{[^}]+\}|\\n|\\r|\\t)")
        .expect("valid placeholder regex")
});

#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("{provider} API key rejected: {message}")]
    InvalidApiKey {
        provider: ProviderId,
        message: String,
    },
    #[error("{provider} model not allowed: {model_id} ({message})")]
    ModelForbiddenOrNotFound {
        provider: ProviderId,
        model_id: String,
        message: String,
    },
    #[error("{provider} quota or plan error: {message}")]
    QuotaOrPlanError {
        provider: ProviderId,
        message: String,
    },
    #[error("{provider} network/http error: {message}")]
    NetworkOrHttp {
        provider: ProviderId,
        message: String,
    },
    #[error("{provider} rate limited: {message}")]
    RateLimited {
        provider: ProviderId,
        message: String,
        retry_after_ms: Option<u64>,
        used_server_hint: bool,
    },
    #[error("placeholder mismatch: {0:?}")]
    PlaceholderMismatch(Vec<String>),
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

#[derive(Debug, Clone, Copy)]
struct RetryAdvice {
    delay_ms: u64,
    used_server_hint: bool,
}

fn map_translation_http_error(
    provider: ProviderId,
    model_id: &str,
    status: StatusCode,
    body: String,
    retry_advice: Option<RetryAdvice>,
) -> TranslationError {
    let message = if body.trim().is_empty() {
        status.to_string()
    } else {
        body
    };
    let lowered = message.to_ascii_lowercase();

    if status == StatusCode::UNAUTHORIZED {
        return TranslationError::InvalidApiKey { provider, message };
    }

    if status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::SERVICE_UNAVAILABLE {
        let delay_ms = retry_advice.map(|advice| advice.delay_ms);
        let used_server_hint = retry_advice
            .map(|advice| advice.used_server_hint)
            .unwrap_or(false);
        return TranslationError::RateLimited {
            provider,
            message,
            retry_after_ms: delay_ms,
            used_server_hint,
        };
    }

    if status == StatusCode::FORBIDDEN || status == StatusCode::NOT_FOUND {
        if lowered.contains("insufficient_quota")
            || lowered.contains("insufficient quota")
            || lowered.contains("plan required")
        {
            return TranslationError::QuotaOrPlanError { provider, message };
        }

        return TranslationError::ModelForbiddenOrNotFound {
            provider,
            model_id: model_id.to_string(),
            message,
        };
    }

    if lowered.contains("insufficient_quota") || lowered.contains("plan required") {
        return TranslationError::QuotaOrPlanError { provider, message };
    }

    TranslationError::NetworkOrHttp { provider, message }
}

fn parse_retry_after(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(seconds) = trimmed.parse::<f64>() {
        if seconds.is_finite() && seconds >= 0.0 {
            let millis = (seconds * 1000.0).round();
            if millis >= 0.0 {
                return Some(millis as u64);
            }
        }
    }

    None
}

fn parse_hint_from_body(body: &str) -> Option<RetryAdvice> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return None;
    };

    if let Some(ms) = value.get("estimated_wait_ms").and_then(|v| v.as_u64()) {
        return Some(RetryAdvice {
            delay_ms: ms,
            used_server_hint: true,
        });
    }

    if let Some(seconds) = value.get("retry_after").and_then(|v| v.as_f64()) {
        if seconds.is_finite() && seconds >= 0.0 {
            let millis = (seconds * 1000.0).round();
            if millis >= 0.0 {
                return Some(RetryAdvice {
                    delay_ms: millis as u64,
                    used_server_hint: true,
                });
            }
        }
    }

    None
}

fn extract_retry_advice(headers: &HeaderMap, body: &str) -> Option<RetryAdvice> {
    if let Some(value) = headers.get("x-server-hint-ms") {
        if let Ok(text) = value.to_str() {
            if let Ok(ms) = text.trim().parse::<u64>() {
                return Some(RetryAdvice {
                    delay_ms: ms,
                    used_server_hint: true,
                });
            }
        }
    }

    if let Some(value) = headers.get("retry-after") {
        if let Ok(text) = value.to_str() {
            if let Some(ms) = parse_retry_after(text) {
                return Some(RetryAdvice {
                    delay_ms: ms,
                    used_server_hint: true,
                });
            }
        }
    }

    parse_hint_from_body(body)
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
        return Err(TranslationError::NetworkOrHttp {
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
        .map_err(|err| TranslationError::NetworkOrHttp {
            provider: ProviderId::Gemini,
            message: err.to_string(),
        })?;

    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response
        .bytes()
        .await
        .map_err(|err| TranslationError::NetworkOrHttp {
            provider: ProviderId::Gemini,
            message: err.to_string(),
        })?;

    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes).to_string();
        let retry_advice = extract_retry_advice(&headers, &body);
        return Err(map_translation_http_error(
            ProviderId::Gemini,
            trimmed_model,
            status,
            body,
            retry_advice,
        ));
    }

    let parsed: GeminiResponse =
        serde_json::from_slice(&bytes).map_err(|err| TranslationError::NetworkOrHttp {
            provider: ProviderId::Gemini,
            message: err.to_string(),
        })?;
    let text = parsed
        .candidates
        .and_then(|candidates| candidates.into_iter().next())
        .and_then(|candidate| candidate.content)
        .and_then(|content| content.parts.and_then(|parts| parts.into_iter().next()))
        .and_then(|part| part.text)
        .ok_or_else(|| TranslationError::NetworkOrHttp {
            provider: ProviderId::Gemini,
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
        return Err(TranslationError::NetworkOrHttp {
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
        .map_err(|err| TranslationError::NetworkOrHttp {
            provider: ProviderId::Gpt,
            message: err.to_string(),
        })?;

    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response
        .bytes()
        .await
        .map_err(|err| TranslationError::NetworkOrHttp {
            provider: ProviderId::Gpt,
            message: err.to_string(),
        })?;

    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes).to_string();
        let retry_advice = extract_retry_advice(&headers, &body);
        return Err(map_translation_http_error(
            ProviderId::Gpt,
            trimmed_model,
            status,
            body,
            retry_advice,
        ));
    }

    let parsed: OpenAiResponse =
        serde_json::from_slice(&bytes).map_err(|err| TranslationError::NetworkOrHttp {
            provider: ProviderId::Gpt,
            message: err.to_string(),
        })?;
    let text = parsed
        .choices
        .into_iter()
        .find_map(|choice| choice.message.and_then(|message| message.content))
        .ok_or_else(|| TranslationError::NetworkOrHttp {
            provider: ProviderId::Gpt,
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
        return Err(TranslationError::NetworkOrHttp {
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
        .map_err(|err| TranslationError::NetworkOrHttp {
            provider: ProviderId::Claude,
            message: err.to_string(),
        })?;

    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response
        .bytes()
        .await
        .map_err(|err| TranslationError::NetworkOrHttp {
            provider: ProviderId::Claude,
            message: err.to_string(),
        })?;

    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes).to_string();
        let retry_advice = extract_retry_advice(&headers, &body);
        return Err(map_translation_http_error(
            ProviderId::Claude,
            trimmed_model,
            status,
            body,
            retry_advice,
        ));
    }

    let parsed: AnthropicResponse =
        serde_json::from_slice(&bytes).map_err(|err| TranslationError::NetworkOrHttp {
            provider: ProviderId::Claude,
            message: err.to_string(),
        })?;
    let text = parsed
        .content
        .unwrap_or_default()
        .into_iter()
        .find_map(|block| block.text)
        .ok_or_else(|| TranslationError::NetworkOrHttp {
            provider: ProviderId::Claude,
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
        return Err(TranslationError::NetworkOrHttp {
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
        .map_err(|err| TranslationError::NetworkOrHttp {
            provider: ProviderId::Grok,
            message: err.to_string(),
        })?;

    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response
        .bytes()
        .await
        .map_err(|err| TranslationError::NetworkOrHttp {
            provider: ProviderId::Grok,
            message: err.to_string(),
        })?;

    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes).to_string();
        let retry_advice = extract_retry_advice(&headers, &body);
        return Err(map_translation_http_error(
            ProviderId::Grok,
            trimmed_model,
            status,
            body,
            retry_advice,
        ));
    }

    let parsed: OpenAiResponse =
        serde_json::from_slice(&bytes).map_err(|err| TranslationError::NetworkOrHttp {
            provider: ProviderId::Grok,
            message: err.to_string(),
        })?;
    let text = parsed
        .choices
        .into_iter()
        .find_map(|choice| choice.message.and_then(|message| message.content))
        .ok_or_else(|| TranslationError::NetworkOrHttp {
            provider: ProviderId::Grok,
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
