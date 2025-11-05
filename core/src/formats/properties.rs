/// Properties (Java) format handler with scanner integration
use super::{FileFormat, FormatError, FormatHandler, TranslatableEntry, TranslationResult};
use crate::scanners::PropertiesScanner;
use std::collections::HashMap;

pub struct PropertiesHandler {
    scanner: PropertiesScanner,
}

impl PropertiesHandler {
    pub fn new() -> Self {
        Self {
            scanner: PropertiesScanner::new(),
        }
    }
}

impl FormatHandler for PropertiesHandler {
    fn extract(&self, content: &str) -> Result<Vec<TranslatableEntry>, FormatError> {
        let entries = self.scanner.parse_file(content);
        
        let mut translatable = Vec::new();
        
        for entry in entries {
            let mut metadata = HashMap::new();
            metadata.insert("line".to_string(), entry.line.to_string());
            
            translatable.push(TranslatableEntry {
                key: entry.key.clone(),
                source: entry.value.clone(),
                context: Some(format!("line {}", entry.line)),
                metadata,
            });
        }
        
        Ok(translatable)
    }

    fn merge(
        &self,
        original: &str,
        translations: &TranslationResult,
    ) -> Result<String, FormatError> {
        // Build a map of translated values by key
        let mut translation_map: HashMap<String, String> = HashMap::new();
        for t in &translations.translated {
            translation_map.insert(t.key.clone(), t.target.clone());
        }
        
        // Parse original and rebuild with translations
        let mut result = String::new();
        let lines: Vec<&str> = original.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();
            
            // Preserve empty lines and comments as-is
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                result.push_str(line);
                result.push('\n');
                i += 1;
                continue;
            }
            
            // Try to parse key=value
            if let Some((key, _old_value)) = Self::parse_key_value(line) {
                // Check if we have a translation for this key
                if let Some(new_value) = translation_map.get(key) {
                    // Rebuild the line with translated value
                    result.push_str(key);
                    result.push('=');
                    result.push_str(new_value);
                    result.push('\n');
                } else {
                    // No translation, keep original
                    result.push_str(line);
                    result.push('\n');
                }
                i += 1;
            } else {
                // Not a key=value line, keep as-is
                result.push_str(line);
                result.push('\n');
                i += 1;
            }
        }
        
        Ok(result)
    }

    fn format(&self) -> FileFormat {
        FileFormat::Properties
    }
}

impl PropertiesHandler {
    fn parse_key_value(line: &str) -> Option<(&str, &str)> {
        // Find first unescaped = or :
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        let mut escaped = false;
        
        while i < chars.len() {
            if escaped {
                escaped = false;
                i += 1;
                continue;
            }
            
            if chars[i] == '\\' {
                escaped = true;
                i += 1;
                continue;
            }
            
            if chars[i] == '=' || chars[i] == ':' {
                let key = line[..i].trim();
                let value = if i + 1 < line.len() {
                    line[i + 1..].trim_start()
                } else {
                    ""
                };
                return Some((key, value));
            }
            
            i += 1;
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{TranslatedEntry, FailedEntry};

    #[test]
    fn test_extract_properties() {
        let handler = PropertiesHandler::new();
        let content = r#"
# Comment
message=Hello World
count=You have %d items
"#;
        
        let entries = handler.extract(content).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "message");
        assert_eq!(entries[0].source, "Hello World");
    }

    #[test]
    fn test_merge_properties() {
        let handler = PropertiesHandler::new();
        let original = r#"# Comment
message=Hello
count=Items: %d
"#;
        
        let result = TranslationResult {
            translated: vec![
                TranslatedEntry {
                    key: "message".to_string(),
                    source: "Hello".to_string(),
                    target: "안녕하세요".to_string(),
                },
                TranslatedEntry {
                    key: "count".to_string(),
                    source: "Items: %d".to_string(),
                    target: "항목: %d".to_string(),
                },
            ],
            failed: vec![],
        };
        
        let merged = handler.merge(original, &result).unwrap();
        assert!(merged.contains("message=안녕하세요"));
        assert!(merged.contains("count=항목: %d"));
        assert!(merged.contains("# Comment"));
    }
}

