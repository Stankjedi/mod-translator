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
        ProviderId::Gemini => validate_gemini(&trimmed_key).await,
        ProviderId::Gpt => validate_openai(&trimmed_key).await,
        ProviderId::Claude => validate_anthropic(&trimmed_key).await,
        ProviderId::Grok => validate_grok(&trimmed_key).await,
    };

    Ok(result)
}

async fn validate_openai(api_key: &str) -> ProviderValidationResult {
    let response = VALIDATION_CLIENT
        .get("https://api.openai.com/v1/models")
        .bearer_auth(api_key)
        .send()
        .await;

    match response {
        Ok(res) => {
            let status = res.status();
            if status == StatusCode::UNAUTHORIZED {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::Unauthorized,
                    models: Vec::new(),
                };
            }
            if status == StatusCode::FORBIDDEN {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::Forbidden,
                    models: Vec::new(),
                };
            }
            if !status.is_success() {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::NetworkError,
                    models: Vec::new(),
                };
            }

            match res.json::<OpenAiModelList>().await {
                Ok(listing) => {
                    let models = listing
                        .data
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|entry| entry.id)
                        .filter(|id| is_openai_chat_model(id))
                        .map(|id| id.trim().to_string())
                        .filter(|id| !id.is_empty())
                        .collect::<Vec<_>>();

                    ProviderValidationResult {
                        validation_status: KeyValidationState::Valid,
                        models: dedupe_and_sort(models),
                    }
                }
                Err(_) => ProviderValidationResult {
                    validation_status: KeyValidationState::NetworkError,
                    models: Vec::new(),
                },
            }
        }
        Err(_) => ProviderValidationResult {
            validation_status: KeyValidationState::NetworkError,
            models: Vec::new(),
        },
    }
}

async fn validate_anthropic(api_key: &str) -> ProviderValidationResult {
    let response = VALIDATION_CLIENT
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await;

    match response {
        Ok(res) => {
            let status = res.status();
            if status == StatusCode::UNAUTHORIZED {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::Unauthorized,
                    models: Vec::new(),
                };
            }
            if status == StatusCode::FORBIDDEN {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::Forbidden,
                    models: Vec::new(),
                };
            }
            if !status.is_success() {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::NetworkError,
                    models: Vec::new(),
                };
            }

            match res.json::<AnthropicModelList>().await {
                Ok(listing) => {
                    let models = listing
                        .data
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|entry| entry.id)
                        .filter(|id| id.starts_with("claude"))
                        .map(|id| id.trim().to_string())
                        .filter(|id| !id.is_empty())
                        .collect::<Vec<_>>();

                    ProviderValidationResult {
                        validation_status: KeyValidationState::Valid,
                        models: dedupe_and_sort(models),
                    }
                }
                Err(_) => ProviderValidationResult {
                    validation_status: KeyValidationState::NetworkError,
                    models: Vec::new(),
                },
            }
        }
        Err(_) => ProviderValidationResult {
            validation_status: KeyValidationState::NetworkError,
            models: Vec::new(),
        },
    }
}

async fn validate_gemini(api_key: &str) -> ProviderValidationResult {
    let mut url = Url::parse("https://generativelanguage.googleapis.com/v1beta/models")
        .expect("valid gemini model list url");
    url.query_pairs_mut().append_pair("key", api_key);

    let response = VALIDATION_CLIENT.get(url).send().await;

    match response {
        Ok(res) => {
            let status = res.status();
            if status == StatusCode::UNAUTHORIZED {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::Unauthorized,
                    models: Vec::new(),
                };
            }
            if status == StatusCode::FORBIDDEN {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::Forbidden,
                    models: Vec::new(),
                };
            }
            if !status.is_success() {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::NetworkError,
                    models: Vec::new(),
                };
            }

            match res.json::<GeminiModelList>().await {
                Ok(listing) => {
                    let models = listing
                        .models
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|model| model.supports_generate_content())
                        .filter_map(|model| model.normalized_name())
                        .collect::<Vec<_>>();

                    ProviderValidationResult {
                        validation_status: KeyValidationState::Valid,
                        models: dedupe_and_sort(models),
                    }
                }
                Err(_) => ProviderValidationResult {
                    validation_status: KeyValidationState::NetworkError,
                    models: Vec::new(),
                },
            }
        }
        Err(_) => ProviderValidationResult {
            validation_status: KeyValidationState::NetworkError,
            models: Vec::new(),
        },
    }
}

async fn validate_grok(api_key: &str) -> ProviderValidationResult {
    let response = VALIDATION_CLIENT
        .get("https://api.x.ai/v1/models")
        .bearer_auth(api_key)
        .send()
        .await;

    match response {
        Ok(res) => {
            let status = res.status();
            if status == StatusCode::UNAUTHORIZED {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::Unauthorized,
                    models: Vec::new(),
                };
            }
            if status == StatusCode::FORBIDDEN {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::Forbidden,
                    models: Vec::new(),
                };
            }
            if status == StatusCode::NOT_FOUND {
                return validate_grok_via_completion(api_key).await;
            }
            if !status.is_success() {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::NetworkError,
                    models: Vec::new(),
                };
            }

            match res.json::<GrokModelList>().await {
                Ok(listing) => {
                    let models = listing
                        .data
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|entry| entry.id)
                        .filter(|id| id.starts_with("grok"))
                        .map(|id| id.trim().to_string())
                        .filter(|id| !id.is_empty())
                        .collect::<Vec<_>>();

                    ProviderValidationResult {
                        validation_status: KeyValidationState::Valid,
                        models: dedupe_and_sort(models),
                    }
                }
                Err(_) => ProviderValidationResult {
                    validation_status: KeyValidationState::NetworkError,
                    models: Vec::new(),
                },
            }
        }
        Err(_) => ProviderValidationResult {
            validation_status: KeyValidationState::NetworkError,
            models: Vec::new(),
        },
    }
}

async fn validate_grok_via_completion(api_key: &str) -> ProviderValidationResult {
    let response = VALIDATION_CLIENT
        .post("https://api.x.ai/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": "grok-4-fast",
            "messages": [
                {
                    "role": "user",
                    "content": "ping",
                }
            ],
            "max_tokens": 1,
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            let status = res.status();
            if status == StatusCode::UNAUTHORIZED {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::Unauthorized,
                    models: Vec::new(),
                };
            }
            if status == StatusCode::FORBIDDEN {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::Forbidden,
                    models: Vec::new(),
                };
            }
            if !status.is_success() {
                return ProviderValidationResult {
                    validation_status: KeyValidationState::NetworkError,
                    models: Vec::new(),
                };
            }

            match res.json::<GrokCompletionResponse>().await {
                Ok(payload) => {
                    let mut models = Vec::new();
                    if let Some(model) = payload.model {
                        let trimmed = model.trim();
                        if !trimmed.is_empty() {
                            models.push(trimmed.to_string());
                        }
                    }
                    if models.is_empty() {
                        if let Some(choice_model) = payload.choice_model() {
                            models.push(choice_model);
                        }
                    }
                    if models.is_empty() {
                        models.push("grok-4-fast".to_string());
                    }
                    ProviderValidationResult {
                        validation_status: KeyValidationState::Valid,
                        models: dedupe_and_sort(models),
                    }
                }
                Err(_) => ProviderValidationResult {
                    validation_status: KeyValidationState::NetworkError,
                    models: Vec::new(),
                },
            }
        }
        Err(_) => ProviderValidationResult {
            validation_status: KeyValidationState::NetworkError,
            models: Vec::new(),
        },
    }
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

#[derive(Debug, serde::Deserialize)]
struct GrokCompletionResponse {
    model: Option<String>,
    #[serde(default)]
    choices: Vec<GrokChoice>,
}

impl GrokCompletionResponse {
    fn choice_model(&self) -> Option<String> {
        self.choices
            .iter()
            .find_map(|choice| choice.model.as_ref().map(|model| model.trim().to_string()))
            .filter(|model| !model.is_empty())
    }
}

#[derive(Debug, serde::Deserialize)]
struct GrokChoice {
    model: Option<String>,
}
