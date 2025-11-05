/// Configuration for the translation system
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorOptions {
    pub enable_autofix: bool,
    pub retry_on_fail: bool,
    pub retry_limit: usize,
    pub strict_pairing: bool,
    pub preserve_percent_binding: bool,
    pub report_download: bool,
    pub jsonl_logging: bool,
}

impl Default for ValidatorOptions {
    fn default() -> Self {
        Self {
            enable_autofix: true,
            retry_on_fail: true,
            retry_limit: 1,
            strict_pairing: true,
            preserve_percent_binding: true,
            report_download: true,
            jsonl_logging: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiOptions {
    pub show_failed_line: bool,
    pub allow_clipboard_copy: bool,
    pub allow_autofix_retry: bool,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            show_failed_line: true,
            allow_clipboard_copy: true,
            allow_autofix_retry: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatorConfig {
    pub validator: ValidatorOptions,
    pub ui: UiOptions,
}

impl Default for TranslatorConfig {
    fn default() -> Self {
        Self {
            validator: ValidatorOptions::default(),
            ui: UiOptions::default(),
        }
    }
}

impl TranslatorConfig {
    /// Load configuration from YAML file
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config: {}", e))
    }

    /// Save configuration to YAML file
    pub fn to_yaml_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        fs::write(path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))
    }

    /// Load from JSON string (for UI integration)
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse JSON config: {}", e))
    }

    /// Convert to JSON string (for UI integration)
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize to JSON: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TranslatorConfig::default();
        assert!(config.validator.enable_autofix);
        assert_eq!(config.validator.retry_limit, 1);
        assert!(config.ui.show_failed_line);
    }

    #[test]
    fn test_json_serialization() {
        let config = TranslatorConfig::default();
        let json = config.to_json().unwrap();
        let deserialized = TranslatorConfig::from_json(&json).unwrap();
        
        assert_eq!(config.validator.enable_autofix, deserialized.validator.enable_autofix);
        assert_eq!(config.validator.retry_limit, deserialized.validator.retry_limit);
    }
}
