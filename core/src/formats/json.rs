/// JSON format handler
/// Preserves structure, only translates string values

use super::{FileFormat, FormatError, FormatHandler, TranslatableEntry, TranslationResult};
use serde_json::Value;

pub struct JsonHandler;

impl JsonHandler {
    pub fn new() -> Self {
        Self
    }

    fn extract_from_value(
        &self,
        value: &Value,
        path: &str,
        entries: &mut Vec<TranslatableEntry>,
    ) {
        match value {
            Value::String(s) if !s.is_empty() && is_translatable(s) => {
                entries.push(TranslatableEntry {
                    key: path.to_string(),
                    source: s.clone(),
                    context: Some(path.to_string()),
                    metadata: Default::default(),
                });
            }
            Value::Object(map) => {
                for (key, val) in map {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    self.extract_from_value(val, &new_path, entries);
                }
            }
            Value::Array(arr) => {
                for (idx, val) in arr.iter().enumerate() {
                    let new_path = format!("{}[{}]", path, idx);
                    self.extract_from_value(val, &new_path, entries);
                }
            }
            _ => {}
        }
    }

    fn apply_translations(
        &self,
        value: &mut Value,
        path: &str,
        translations: &std::collections::HashMap<String, String>,
    ) {
        match value {
            Value::String(s) => {
                if let Some(translated) = translations.get(path) {
                    *s = translated.clone();
                }
            }
            Value::Object(map) => {
                for (key, val) in map.iter_mut() {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    self.apply_translations(val, &new_path, translations);
                }
            }
            Value::Array(arr) => {
                for (idx, val) in arr.iter_mut().enumerate() {
                    let new_path = format!("{}[{}]", path, idx);
                    self.apply_translations(val, &new_path, translations);
                }
            }
            _ => {}
        }
    }
}

impl FormatHandler for JsonHandler {
    fn extract(&self, content: &str) -> Result<Vec<TranslatableEntry>, FormatError> {
        let value: Value = serde_json::from_str(content)
            .map_err(|e| FormatError::ParseError(format!("JSON parse error: {}", e)))?;

        let mut entries = Vec::new();
        self.extract_from_value(&value, "", &mut entries);
        Ok(entries)
    }

    fn merge(
        &self,
        original: &str,
        translations: &TranslationResult,
    ) -> Result<String, FormatError> {
        let mut value: Value = serde_json::from_str(original)
            .map_err(|e| FormatError::ParseError(format!("JSON parse error: {}", e)))?;

        let mut translation_map = std::collections::HashMap::new();
        for entry in &translations.translated {
            translation_map.insert(entry.key.clone(), entry.target.clone());
        }

        self.apply_translations(&mut value, "", &translation_map);

        serde_json::to_string_pretty(&value)
            .map_err(|e| FormatError::SerializationError(format!("JSON serialize error: {}", e)))
    }

    fn format(&self) -> FileFormat {
        FileFormat::Json
    }
}

/// Check if a string is translatable (not a technical identifier)
fn is_translatable(s: &str) -> bool {
    // Skip if it looks like a path, URL, or identifier
    if s.starts_with('/') || s.starts_with("http") || s.starts_with("data/") {
        return false;
    }

    // Check if string has alphabetic chars and if any are lowercase
    let mut has_alpha = false;
    let mut has_lower = false;
    
    for c in s.chars() {
        if c.is_alphabetic() {
            has_alpha = true;
            if c.is_lowercase() {
                has_lower = true;
                break; // Found lowercase, no need to continue
            }
        }
    }
    
    // Must have alphabetic characters, and at least some lowercase
    // (all uppercase likely means constant)
    has_alpha && (has_lower || s.len() <= 3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::TranslatedEntry;

    #[test]
    fn extracts_string_values() {
        let handler = JsonHandler::new();
        let json = r#"{"message": "Hello", "count": 42, "nested": {"text": "World"}}"#;
        let entries = handler.extract(json).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].source, "Hello");
        assert_eq!(entries[1].source, "World");
    }

    #[test]
    fn preserves_structure_in_merge() {
        let handler = JsonHandler::new();
        let json = r#"{"message": "Hello", "count": 42}"#;
        
        let result = TranslationResult {
            translated: vec![TranslatedEntry {
                key: "message".to_string(),
                source: "Hello".to_string(),
                target: "Bonjour".to_string(),
            }],
            failed: vec![],
        };
        
        let merged = handler.merge(json, &result).unwrap();
        let value: Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(value["message"], "Bonjour");
        assert_eq!(value["count"], 42);
    }
}
