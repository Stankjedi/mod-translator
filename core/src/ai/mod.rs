use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

static PLACEHOLDER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\{\d+\}|%\d*\$?s|%\d*\$?d|\$\{[^}]+\}|\\n|\\r|\\t)")
        .expect("valid placeholder regex")
});

#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("translator reported an error: {0}")]
    Failure(String),
    #[error("output lost placeholders: {0:?}")]
    PlaceholderMismatch(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationDomain {
    Ui,
    Dialog,
    System,
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationStyle {
    Neutral,
    Game,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateOptions {
    pub source_lang: Option<String>,
    pub target_lang: String,
    pub domain: Option<TranslationDomain>,
    pub style: Option<TranslationStyle>,
}

impl TranslateOptions {
    pub fn for_preview(source: &str, target: &str) -> Self {
        Self {
            source_lang: Some(source.to_string()),
            target_lang: target.to_string(),
            domain: Some(TranslationDomain::Ui),
            style: Some(TranslationStyle::Game),
        }
    }
}

impl Default for TranslateOptions {
    fn default() -> Self {
        Self {
            source_lang: Some("en".into()),
            target_lang: "en".into(),
            domain: Some(TranslationDomain::Ui),
            style: Some(TranslationStyle::Neutral),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderAuth {
    pub gemini: Option<String>,
    pub gpt: Option<String>,
    pub claude: Option<String>,
    pub grok: Option<String>,
}

pub trait Translator: Send {
    fn name(&self) -> &'static str;
    fn translate_batch(
        &mut self,
        inputs: &[String],
        options: &TranslateOptions,
    ) -> Result<Vec<String>, TranslationError>;

    fn translate_preview(
        &mut self,
        input: &str,
        options: &TranslateOptions,
    ) -> Result<String, TranslationError> {
        let outputs = self.translate_batch(&[input.to_string()], options)?;
        Ok(outputs.into_iter().next().unwrap_or_default())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslatorKind {
    Gemini,
    Gpt,
    Claude,
    Grok,
}

impl TranslatorKind {
    pub fn label(&self) -> &'static str {
        match self {
            TranslatorKind::Gemini => "Gemini",
            TranslatorKind::Gpt => "GPT",
            TranslatorKind::Claude => "Claude",
            TranslatorKind::Grok => "Grok",
        }
    }

    pub fn build(self) -> Box<dyn Translator> {
        self.build_with_auth(&ProviderAuth::default())
    }

    pub fn build_with_auth(self, auth: &ProviderAuth) -> Box<dyn Translator> {
        match self {
            TranslatorKind::Gemini => {
                Box::new(GeminiTranslator::default().with_api_key(auth.gemini.clone()))
            }
            TranslatorKind::Gpt => {
                Box::new(GptTranslator::default().with_api_key(auth.gpt.clone()))
            }
            TranslatorKind::Claude => {
                Box::new(ClaudeTranslator::default().with_api_key(auth.claude.clone()))
            }
            TranslatorKind::Grok => {
                Box::new(GrokTranslator::default().with_api_key(auth.grok.clone()))
            }
        }
    }
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

#[derive(Debug, Default)]
pub struct GeminiTranslator {
    invocation_count: u32,
    api_key: Option<String>,
}

impl Translator for GeminiTranslator {
    fn name(&self) -> &'static str {
        "Gemini"
    }

    fn translate_batch(
        &mut self,
        inputs: &[String],
        options: &TranslateOptions,
    ) -> Result<Vec<String>, TranslationError> {
        self.invocation_count += 1;
        let mut outputs = Vec::with_capacity(inputs.len());
        for input in inputs {
            let key_status = if self.api_key.is_some() {
                "with-key"
            } else {
                "no-key"
            };
            let output = format!(
                "[Gemini#{:03} {key_status}] {} -> {} ({:?}/{:?}): {}",
                self.invocation_count,
                options.source_lang.as_deref().unwrap_or("unknown"),
                options.target_lang,
                options.domain,
                options.style,
                input
            );
            ensure_placeholder_integrity(input, &output)?;
            outputs.push(output);
        }

        Ok(outputs)
    }
}

#[derive(Debug, Default)]
pub struct GptTranslator {
    invocation_count: u32,
    api_key: Option<String>,
}

impl Translator for GptTranslator {
    fn name(&self) -> &'static str {
        "GPT"
    }

    fn translate_batch(
        &mut self,
        inputs: &[String],
        options: &TranslateOptions,
    ) -> Result<Vec<String>, TranslationError> {
        self.invocation_count += 1;
        let mut outputs = Vec::with_capacity(inputs.len());
        for input in inputs {
            let key_status = if self.api_key.is_some() {
                "with-key"
            } else {
                "no-key"
            };
            let domain_label = options
                .domain
                .as_ref()
                .map(|domain| format!("{:?}", domain))
                .unwrap_or_else(|| "domain:none".into());
            let output = format!(
                "[GPT#{:03} {key_status}] {} -> {} [{}]: {}",
                self.invocation_count,
                options.source_lang.as_deref().unwrap_or("unknown"),
                options.target_lang,
                domain_label,
                input
            );
            ensure_placeholder_integrity(input, &output)?;
            outputs.push(output);
        }

        Ok(outputs)
    }
}

#[derive(Debug, Default)]
pub struct ClaudeTranslator {
    batches_processed: u32,
    api_key: Option<String>,
}

impl Translator for ClaudeTranslator {
    fn name(&self) -> &'static str {
        "Claude"
    }

    fn translate_batch(
        &mut self,
        inputs: &[String],
        options: &TranslateOptions,
    ) -> Result<Vec<String>, TranslationError> {
        self.batches_processed += 1;
        let mut outputs = Vec::with_capacity(inputs.len());
        for input in inputs {
            let key_status = if self.api_key.is_some() {
                "with-key"
            } else {
                "no-key"
            };
            let output = format!(
                "[Claude batch {} {key_status}] {} -> {} :: {}",
                self.batches_processed,
                options.source_lang.as_deref().unwrap_or("unknown"),
                options.target_lang,
                input
            );
            ensure_placeholder_integrity(input, &output)?;
            outputs.push(output);
        }

        Ok(outputs)
    }
}

#[derive(Debug, Default)]
pub struct GrokTranslator {
    previews_generated: u32,
    api_key: Option<String>,
}

impl Translator for GrokTranslator {
    fn name(&self) -> &'static str {
        "Grok"
    }

    fn translate_batch(
        &mut self,
        inputs: &[String],
        options: &TranslateOptions,
    ) -> Result<Vec<String>, TranslationError> {
        self.previews_generated += 1;
        let mut outputs = Vec::with_capacity(inputs.len());
        for input in inputs {
            let key_status = if self.api_key.is_some() {
                "with-key"
            } else {
                "no-key"
            };
            let output = format!(
                "[Grok run {} {key_status}] {}>{}: {}",
                self.previews_generated,
                options.source_lang.as_deref().unwrap_or("unknown"),
                options.target_lang,
                input
            );
            ensure_placeholder_integrity(input, &output)?;
            outputs.push(output);
        }

        Ok(outputs)
    }
}

fn normalize_api_key(key: Option<String>) -> Option<String> {
    key.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

impl ProviderAuth {
    pub fn set_key(&mut self, provider_id: &str, key: Option<String>) {
        let slot = match provider_id {
            "gemini" => &mut self.gemini,
            "gpt" => &mut self.gpt,
            "claude" => &mut self.claude,
            "grok" => &mut self.grok,
            _ => return,
        };

        *slot = normalize_api_key(key);
    }

    pub fn key(&self, provider_id: &str) -> Option<&String> {
        match provider_id {
            "gemini" => self.gemini.as_ref(),
            "gpt" => self.gpt.as_ref(),
            "claude" => self.claude.as_ref(),
            "grok" => self.grok.as_ref(),
            _ => None,
        }
    }
}

impl GeminiTranslator {
    pub fn with_api_key(mut self, key: Option<String>) -> Self {
        self.api_key = normalize_api_key(key);
        self
    }
}

impl GptTranslator {
    pub fn with_api_key(mut self, key: Option<String>) -> Self {
        self.api_key = normalize_api_key(key);
        self
    }
}

impl ClaudeTranslator {
    pub fn with_api_key(mut self, key: Option<String>) -> Self {
        self.api_key = normalize_api_key(key);
        self
    }
}

impl GrokTranslator {
    pub fn with_api_key(mut self, key: Option<String>) -> Self {
        self.api_key = normalize_api_key(key);
        self
    }
}
