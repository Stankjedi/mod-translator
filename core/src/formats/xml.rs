/// XML format handler
/// Preserves tags and attributes, only translates text nodes

use super::{FileFormat, FormatError, FormatHandler, TranslatableEntry, TranslationResult};
use once_cell::sync::Lazy;
use regex::Regex;

static TAG_CONTENT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r">([^<]+)<").expect("valid tag content regex")
});

pub struct XmlHandler;

impl XmlHandler {
    pub fn new() -> Self {
        Self
    }
}

impl FormatHandler for XmlHandler {
    fn extract(&self, content: &str) -> Result<Vec<TranslatableEntry>, FormatError> {
        // Placeholder: Basic regex-based extraction for MVP
        let mut entries = Vec::new();
        
        for (idx, captures) in TAG_CONTENT_RE.captures_iter(content).enumerate() {
            if let Some(text) = captures.get(1) {
                let text_str = text.as_str().trim();
                if !text_str.is_empty() && text_str.chars().any(|c| c.is_alphabetic()) {
                    entries.push(TranslatableEntry {
                        key: format!("xml_text_{}", idx),
                        source: text_str.to_string(),
                        context: Some(format!("position {}", idx)),
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
        // Placeholder: Simple replacement for MVP
        let mut result = original.to_string();
        for entry in &translations.translated {
            result = result.replace(&entry.source, &entry.target);
        }
        Ok(result)
    }

    fn format(&self) -> FileFormat {
        FileFormat::Xml
    }
}
