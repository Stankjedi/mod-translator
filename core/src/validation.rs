use crate::ai::ProviderId;
use once_cell::sync::Lazy;
use reqwest::{Client, StatusCode, Url};
use serde::Serialize;
use std::time::Duration;

static VALIDATION_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("failed to build validation client")
});

const OPENAI_FALLBACK_MODELS: &[&str] = &["gpt-4o-mini", "gpt-4o"];
const ANTHROPIC_FALLBACK_MODELS: &[&str] = &[
    "claude-3-5-sonnet-20240620",
    "claude-3-opus-20240229",
    "claude-3-haiku-20240307",
];
const GEMINI_FALLBACK_MODELS: &[&str] = &[
    "gemini-2.5-flash",
    "gemini-2.5-pro",
    "gemini-2.5-flash-lite",
];
const GROK_FALLBACK_MODELS: &[&str] = &["grok-2-1212", "grok-4-fast"];

fn pick_model_candidate(
    model_hint: Option<&str>,
    models: &[String],
    fallbacks: &[&str],
) -> Option<String> {
    if let Some(hint) = model_hint {
        let trimmed = hint.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Some(existing) = models.first() {
        if !existing.trim().is_empty() {
            return Some(existing.clone());
        }
    }

    fallbacks
        .iter()
        .map(|candidate| candidate.trim())
        .find(|candidate| !candidate.is_empty())
        .map(|candidate| candidate.to_string())
}

fn map_failure(status: StatusCode, body: &str) -> KeyValidationState {
    if status == StatusCode::UNAUTHORIZED {
        return KeyValidationState::Unauthorized;
    }
    if status == StatusCode::FORBIDDEN || status == StatusCode::NOT_FOUND {
        return KeyValidationState::Forbidden;
    }

    let lowered = body.to_ascii_lowercase();
    if lowered.contains("insufficient_quota")
        || lowered.contains("insufficient quota")
        || lowered.contains("plan required")
    {
        return KeyValidationState::Forbidden;
    }

    if status == StatusCode::TOO_MANY_REQUESTS && lowered.contains("quota") {
        return KeyValidationState::Forbidden;
    }

    KeyValidationState::NetworkError
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyValidationState {
    Valid,
    Unauthorized,
    Forbidden,
    NetworkError,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderValidationResult {
    pub validation_status: KeyValidationState,
    #[serde(default)]
    pub models: Vec<String>,
}

#[tauri::command]
pub async fn validate_api_key_and_list_models(
    provider: String,
    api_key: String,
    model_hint: Option<String>,
) -> Result<ProviderValidationResult, String> {
    let trimmed_key = api_key.trim().to_string();
    if trimmed_key.is_empty() {
        return Ok(ProviderValidationResult {
            validation_status: KeyValidationState::Unauthorized,
            models: Vec::new(),
        });
    }

    let provider_id = ProviderId::try_from(provider.as_str())
        .map_err(|_| format!("unsupported provider: {provider}"))?;

    let result = match provider_id {
        ProviderId::Gemini => validate_gemini(&trimmed_key, model_hint.as_deref()).await,
        ProviderId::Gpt => validate_openai(&trimmed_key, model_hint.as_deref()).await,
        ProviderId::Claude => validate_anthropic(&trimmed_key, model_hint.as_deref()).await,
        ProviderId::Grok => validate_grok(&trimmed_key, model_hint.as_deref()).await,
    };

    Ok(result)
}

async fn validate_openai(api_key: &str, model_hint: Option<&str>) -> ProviderValidationResult {
    let models = match fetch_openai_models(api_key).await {
        Ok(list) => list,
        Err(status) => {
            return ProviderValidationResult {
                validation_status: status,
                models: Vec::new(),
            };
        }
    };

    let Some(candidate) = pick_model_candidate(model_hint, &models, OPENAI_FALLBACK_MODELS) else {
        return ProviderValidationResult {
            validation_status: KeyValidationState::NetworkError,
            models: Vec::new(),
        };
    };

    match test_openai_model(api_key, &candidate).await {
        Ok(()) => {
            let mut merged = models;
            if !merged.iter().any(|entry| entry == &candidate) {
                merged.push(candidate);
            }
            ProviderValidationResult {
                validation_status: KeyValidationState::Valid,
                models: dedupe_and_sort(merged),
            }
        }
        Err(status) => ProviderValidationResult {
            validation_status: status,
            models: Vec::new(),
        },
    }
}

async fn fetch_openai_models(api_key: &str) -> Result<Vec<String>, KeyValidationState> {
    let response = VALIDATION_CLIENT
        .get("https://api.openai.com/v1/models")
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|_| KeyValidationState::NetworkError)?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(map_failure(status, &body));
    }

    let listing = response
        .json::<OpenAiModelList>()
        .await
        .map_err(|_| KeyValidationState::NetworkError)?;

    let models = listing
        .data
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| entry.id)
        .filter(|id| is_openai_chat_model(id))
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();

    Ok(dedupe_and_sort(models))
}

async fn test_openai_model(api_key: &str, model: &str) -> Result<(), KeyValidationState> {
    let response = VALIDATION_CLIENT
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": "ping"
                }
            ],
            "max_tokens": 1,
            "temperature": 0.0
        }))
        .send()
        .await
        .map_err(|_| KeyValidationState::NetworkError)?;

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = response.text().await.unwrap_or_default();
    Err(map_failure(status, &body))
}

async fn validate_anthropic(api_key: &str, model_hint: Option<&str>) -> ProviderValidationResult {
    let models = match fetch_anthropic_models(api_key).await {
        Ok(list) => list,
        Err(status) => {
            return ProviderValidationResult {
                validation_status: status,
                models: Vec::new(),
            };
        }
    };

    let Some(candidate) = pick_model_candidate(model_hint, &models, ANTHROPIC_FALLBACK_MODELS)
    else {
        return ProviderValidationResult {
            validation_status: KeyValidationState::NetworkError,
            models: Vec::new(),
        };
    };

    match test_anthropic_model(api_key, &candidate).await {
        Ok(()) => {
            let mut merged = models;
            if !merged.iter().any(|entry| entry == &candidate) {
                merged.push(candidate);
            }
            ProviderValidationResult {
                validation_status: KeyValidationState::Valid,
                models: dedupe_and_sort(merged),
            }
        }
        Err(status) => ProviderValidationResult {
            validation_status: status,
            models: Vec::new(),
        },
    }
}

async fn fetch_anthropic_models(api_key: &str) -> Result<Vec<String>, KeyValidationState> {
    let response = VALIDATION_CLIENT
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|_| KeyValidationState::NetworkError)?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(map_failure(status, &body));
    }

    let listing = response
        .json::<AnthropicModelList>()
        .await
        .map_err(|_| KeyValidationState::NetworkError)?;

    let models = listing
        .data
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| entry.id)
        .filter(|id| id.starts_with("claude"))
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();

    Ok(dedupe_and_sort(models))
}

async fn test_anthropic_model(api_key: &str, model: &str) -> Result<(), KeyValidationState> {
    let response = VALIDATION_CLIENT
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": 1,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": "ping"
                        }
                    ]
                }
            ]
        }))
        .send()
        .await
        .map_err(|_| KeyValidationState::NetworkError)?;

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = response.text().await.unwrap_or_default();
    Err(map_failure(status, &body))
}

async fn validate_gemini(api_key: &str, model_hint: Option<&str>) -> ProviderValidationResult {
    let models = match fetch_gemini_models(api_key).await {
        Ok(list) => list,
        Err(status) => {
            return ProviderValidationResult {
                validation_status: status,
                models: Vec::new(),
            };
        }
    };

    let Some(candidate) = pick_model_candidate(model_hint, &models, GEMINI_FALLBACK_MODELS) else {
        return ProviderValidationResult {
            validation_status: KeyValidationState::NetworkError,
            models: Vec::new(),
        };
    };

    match test_gemini_model(api_key, &candidate).await {
        Ok(()) => {
            let mut merged = models;
            if !merged.iter().any(|entry| entry == &candidate) {
                merged.push(candidate);
            }
            ProviderValidationResult {
                validation_status: KeyValidationState::Valid,
                models: dedupe_and_sort(merged),
            }
        }
        Err(status) => ProviderValidationResult {
            validation_status: status,
            models: Vec::new(),
        },
    }
}

async fn fetch_gemini_models(api_key: &str) -> Result<Vec<String>, KeyValidationState> {
    let mut url = Url::parse("https://generativelanguage.googleapis.com/v1beta/models")
        .expect("valid gemini model list url");
    url.query_pairs_mut().append_pair("key", api_key);

    let response = VALIDATION_CLIENT
        .get(url)
        .send()
        .await
        .map_err(|_| KeyValidationState::NetworkError)?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(map_failure(status, &body));
    }

    let listing = response
        .json::<GeminiModelList>()
        .await
        .map_err(|_| KeyValidationState::NetworkError)?;

    let models = listing
        .models
        .unwrap_or_default()
        .into_iter()
        .filter(|model| model.supports_generate_content())
        .filter_map(|model| model.normalized_name())
        .collect::<Vec<_>>();

    Ok(dedupe_and_sort(models))
}

async fn test_gemini_model(api_key: &str, model: &str) -> Result<(), KeyValidationState> {
    let normalized_model = if model.starts_with("models/") {
        model.to_string()
    } else {
        format!("models/{model}")
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/{}:generateContent?key={}",
        normalized_model, api_key
    );

    let response = VALIDATION_CLIENT
        .post(url)
        .json(&serde_json::json!({
            "contents": [
                {
                    "parts": [
                        {
                            "text": "ping"
                        }
                    ]
                }
            ]
        }))
        .send()
        .await
        .map_err(|_| KeyValidationState::NetworkError)?;

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = response.text().await.unwrap_or_default();
    Err(map_failure(status, &body))
}

async fn validate_grok(api_key: &str, model_hint: Option<&str>) -> ProviderValidationResult {
    let models_result = fetch_grok_models(api_key).await;
    let models = match models_result {
        Ok(list) => list,
        Err(KeyValidationState::NetworkError) => Vec::new(),
        Err(status) => {
            return ProviderValidationResult {
                validation_status: status,
                models: Vec::new(),
            };
        }
    };

    let Some(candidate) = pick_model_candidate(model_hint, &models, GROK_FALLBACK_MODELS) else {
        return ProviderValidationResult {
            validation_status: KeyValidationState::NetworkError,
            models: Vec::new(),
        };
    };

    match test_grok_model(api_key, &candidate).await {
        Ok(()) => {
            let mut merged = models;
            if !merged.iter().any(|entry| entry == &candidate) {
                merged.push(candidate);
            }
            ProviderValidationResult {
                validation_status: KeyValidationState::Valid,
                models: dedupe_and_sort(merged),
            }
        }
        Err(status) => ProviderValidationResult {
            validation_status: status,
            models: Vec::new(),
        },
    }
}

async fn fetch_grok_models(api_key: &str) -> Result<Vec<String>, KeyValidationState> {
    let response = VALIDATION_CLIENT
        .get("https://api.x.ai/v1/models")
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|_| KeyValidationState::NetworkError)?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(map_failure(status, &body));
    }

    let listing = response
        .json::<GrokModelList>()
        .await
        .map_err(|_| KeyValidationState::NetworkError)?;

    let models = listing
        .data
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| entry.id)
        .filter(|id| id.starts_with("grok"))
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();

    Ok(dedupe_and_sort(models))
}

async fn test_grok_model(api_key: &str, model: &str) -> Result<(), KeyValidationState> {
    let response = VALIDATION_CLIENT
        .post("https://api.x.ai/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": "ping"
                }
            ],
            "max_tokens": 1
        }))
        .send()
        .await
        .map_err(|_| KeyValidationState::NetworkError)?;

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = response.text().await.unwrap_or_default();
    Err(map_failure(status, &body))
}

fn dedupe_and_sort(models: Vec<String>) -> Vec<String> {
    let mut unique: Vec<String> = models
        .into_iter()
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty())
        .collect();
    unique.sort();
    unique.dedup();
    unique
}

fn is_openai_chat_model(id: &str) -> bool {
    let lowered = id.to_lowercase();
    lowered.starts_with("gpt") || lowered.starts_with('o') || lowered.contains("chat")
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiModelList {
    data: Option<Vec<OpenAiModelEntry>>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiModelEntry {
    id: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct AnthropicModelList {
    data: Option<Vec<AnthropicModelEntry>>,
}

#[derive(Debug, serde::Deserialize)]
struct AnthropicModelEntry {
    id: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct GeminiModelList {
    models: Option<Vec<GeminiModelEntry>>,
}

#[derive(Debug, serde::Deserialize)]
struct GeminiModelEntry {
    name: Option<String>,
    #[serde(default)]
    supported_generation_methods: Vec<String>,
}

impl GeminiModelEntry {
    fn supports_generate_content(&self) -> bool {
        if self.supported_generation_methods.is_empty() {
            return true;
        }
        self.supported_generation_methods
            .iter()
            .any(|method| method.eq_ignore_ascii_case("generateContent"))
    }

    fn normalized_name(&self) -> Option<String> {
        let raw = self.name.as_deref()?.trim();
        if raw.is_empty() {
            return None;
        }
        if let Some(stripped) = raw.strip_prefix("models/") {
            let trimmed = stripped.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        } else {
            Some(raw.to_string())
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct GrokModelList {
    data: Option<Vec<GrokModelEntry>>,
}

#[derive(Debug, serde::Deserialize)]
struct GrokModelEntry {
    id: Option<String>,
}
