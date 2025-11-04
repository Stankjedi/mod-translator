/// INI/CFG format handler
/// Preserves keys and sections, only translates values

use super::{FileFormat, FormatError, FormatHandler, TranslatableEntry, TranslationResult};

pub struct IniHandler;

impl IniHandler {
    pub fn new() -> Self {
        Self
    }
}

impl FormatHandler for IniHandler {
    fn extract(&self, content: &str) -> Result<Vec<TranslatableEntry>, FormatError> {
        let mut entries = Vec::new();
        let mut current_section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            
            // Section header
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = trimmed[1..trimmed.len()-1].to_string();
                continue;
            }
            
            // Key=value pair
            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim();
                let value = trimmed[pos+1..].trim();
                
                if !value.is_empty() && value.chars().any(|c| c.is_alphabetic()) {
                    let full_key = if current_section.is_empty() {
                        key.to_string()
                    } else {
                        format!("{}:{}", current_section, key)
                    };
                    
                    entries.push(TranslatableEntry {
                        key: full_key,
                        source: value.to_string(),
                        context: Some(format!("section: {}", current_section)),
                        metadata: Default::default(),
                    });
                }
            }
        }
        
        Ok(entries)
    }

    fn merge(
        &self,
        original: &str,
        translations: &TranslationResult,
    ) -> Result<String, FormatError> {
        let mut translation_map = std::collections::HashMap::new();
        for entry in &translations.translated {
            translation_map.insert(entry.source.clone(), entry.target.clone());
        }

        let mut result = Vec::new();
        for line in original.lines() {
            let trimmed = line.trim();
            
            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim();
                let value = trimmed[pos+1..].trim();
                
                if let Some(translated) = translation_map.get(value) {
                    result.push(format!("{}={}", key, translated));
                } else {
                    result.push(line.to_string());
                }
            } else {
                result.push(line.to_string());
            }
        }
        
        Ok(result.join("\n"))
    }

    fn format(&self) -> FileFormat {
        FileFormat::Ini
    }
}
