/// Markdown format handler with scanner integration
use super::{FileFormat, FormatError, FormatHandler, TranslatableEntry, TranslationResult};
use crate::scanners::MarkdownScanner;

pub struct MarkdownHandler {
    #[allow(dead_code)]
    scanner: MarkdownScanner,
}

impl MarkdownHandler {
    pub fn new() -> Self {
        Self {
            scanner: MarkdownScanner::new(),
        }
    }
}

impl FormatHandler for MarkdownHandler {
    fn extract(&self, content: &str) -> Result<Vec<TranslatableEntry>, FormatError> {
        let mut entries = Vec::new();
        
        // For markdown, we treat the whole content as one translatable unit
        // In the future, this could be enhanced to split by paragraphs or sections
        let entry = TranslatableEntry {
            key: "content".to_string(),
            source: content.to_string(),
            context: Some("markdown_content".to_string()),
            metadata: std::collections::HashMap::new(),
        };
        
        entries.push(entry);
        
        Ok(entries)
    }

    fn merge(
        &self,
        _original: &str,
        translations: &TranslationResult,
    ) -> Result<String, FormatError> {
        // For simple markdown files, we just use the translated content directly
        if let Some(translated) = translations.translated.first() {
            Ok(translated.target.clone())
        } else {
            // If no translation, return empty or original
            Ok(String::new())
        }
    }

    fn format(&self) -> FileFormat {
        FileFormat::Markdown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::TranslatedEntry;

    #[test]
    fn test_extract_markdown() {
        let handler = MarkdownHandler::new();
        let content = "# Header\n\nSome text with `code`.";
        
        let entries = handler.extract(content).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "content");
        assert!(entries[0].source.contains("Header"));
    }

    #[test]
    fn test_merge_markdown() {
        let handler = MarkdownHandler::new();
        let original = "# Header\n\nOriginal text.";
        
        let result = TranslationResult {
            translated: vec![TranslatedEntry {
                key: "content".to_string(),
                source: original.to_string(),
                target: "# 헤더\n\n번역된 텍스트.".to_string(),
            }],
            failed: vec![],
        };
        
        let merged = handler.merge(original, &result).unwrap();
        assert!(merged.contains("헤더"));
        assert!(merged.contains("번역된"));
    }
}
