use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

static PLACEHOLDER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\{\d+\}|%\d*\$?s|%\d*\$?d|\$\{[^}]+\}|\\n|\\r|\\t)")
        .expect("valid placeholder regex")
});

#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("http error: {0}")]
    Http(String),
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

impl From<reqwest::Error> for TranslationError {
    fn from(value: reqwest::Error) -> Self {
        TranslationError::Http(value.to_string())
    }
}

impl From<serde_json::Error> for TranslationError {
    fn from(value: serde_json::Error) -> Self {
        TranslationError::Http(value.to_string())
    }
}

pub async fn translate_text(
    client: &Client,
    provider: ProviderId,
    api_key: &str,
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
            translate_with_gemini(client, api_key, normalized_input, source_lang, target_lang)
                .await?
        }
        ProviderId::Gpt => {
            translate_with_gpt(client, api_key, normalized_input, source_lang, target_lang).await?
        }
        ProviderId::Claude | ProviderId::Grok => {
            return Err(TranslationError::Provider(format!(
                "{} provider is not implemented yet",
                provider.label()
            )))
        }
    };

    ensure_placeholder_integrity(normalized_input, &translated)?;
    Ok(translated.trim().to_string())
}

async fn translate_with_gemini(
    client: &Client,
    api_key: &str,
    input: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<String, TranslationError> {
    let prompt = format!(
        "Translate the following text from {source_lang} to {target_lang}. \
Preserve any placeholders such as {{0}}, %1$s, or similar tokens exactly as they appear.\n\n{input}"
    );

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={api_key}"
    );
    let response = client
        .post(url)
        .json(&serde_json::json!({
            "contents": [{ "parts": [{ "text": prompt }] }]
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(TranslationError::Http(format!(
            "Gemini API {}: {}",
            status, body
        )));
    }

    let parsed: GeminiResponse = response.json().await?;
    let text = parsed
        .candidates
        .and_then(|mut candidates| candidates.into_iter().next())
        .and_then(|candidate| candidate.content)
        .and_then(|mut content| content.parts.and_then(|mut parts| parts.into_iter().next()))
        .and_then(|part| part.text)
        .ok_or_else(|| {
            TranslationError::Provider("Gemini 응답에서 결과를 찾지 못했습니다.".into())
        })?;

    Ok(text)
}

async fn translate_with_gpt(
    client: &Client,
    api_key: &str,
    input: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<String, TranslationError> {
    let prompt = format!(
        "Translate the following text from {source_lang} to {target_lang}. Preserve all placeholders such as {{0}} or %1$s exactly as they appear. Return only the translated text.\n\n{input}"
    );

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": "gpt-4o-mini",
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
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(TranslationError::Http(format!(
            "OpenAI API {}: {}",
            status, body
        )));
    }

    let parsed: OpenAiResponse = response.json().await?;
    let text = parsed
        .choices
        .into_iter()
        .find_map(|choice| choice.message.and_then(|message| message.content))
        .ok_or_else(|| TranslationError::Provider("GPT 응답에서 결과를 찾지 못했습니다.".into()))?;

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
