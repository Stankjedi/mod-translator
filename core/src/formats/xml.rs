/// XML format handler
/// Preserves tags and attributes, only translates text nodes

use super::{FileFormat, FormatError, FormatHandler, TranslatableEntry, TranslationResult};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

// Matches text content between tags: >text<
static TAG_CONTENT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r">([^<]+)<").expect("valid tag content regex")
});

// Matches CDATA sections
static CDATA_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<!\[CDATA\[([\s\S]*?)\]\]>").expect("valid CDATA regex")
});

// Matches XML comments (should be skipped)
static COMMENT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<!--[\s\S]*?-->").expect("valid comment regex")
});

// Common translatable attribute names
static TRANSLATABLE_ATTRS: &[&str] = &[
    "label", "title", "description", "text", "tooltip",
    "name", "displayName", "caption", "alt", "placeholder",
    "value", "content", "summary", "hint", "message",
];

// Pattern to extract attribute values
static ATTR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(\w+)\s*=\s*"([^"]*)""#).expect("valid attr regex")
});

// Tags that typically don't contain translatable text
static SKIP_TAGS: &[&str] = &[
    "script", "style", "code", "pre", "defName", 
    "texPath", "graphicPath", "soundPath", "icon",
    "shaderType", "graphicClass",
];

// Check if text is likely a file path or ID
static PATH_OR_ID_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Z][a-zA-Z0-9_]*(/[A-Z][a-zA-Z0-9_]*)*$|^\w+\.\w+$|^/|^[a-z]+_[a-z]+(_[a-z]+)*$").expect("valid path regex")
});

pub struct XmlHandler {
    /// Tags to skip when extracting
    skip_tags: Vec<String>,
    /// Attribute names that should be translated
    translatable_attrs: Vec<String>,
}

impl XmlHandler {
    pub fn new() -> Self {
        Self {
            skip_tags: SKIP_TAGS.iter().map(|s| s.to_string()).collect(),
            translatable_attrs: TRANSLATABLE_ATTRS.iter().map(|s| s.to_string()).collect(),
        }
    }
    
    /// Create a handler with custom configuration
    pub fn with_config(skip_tags: Vec<String>, translatable_attrs: Vec<String>) -> Self {
        Self {
            skip_tags,
            translatable_attrs,
        }
    }
    
    /// Check if text appears to be translatable (not code/path/ID)
    fn is_translatable_text(&self, text: &str) -> bool {
        let trimmed = text.trim();
        
        // Empty or whitespace-only
        if trimmed.is_empty() {
            return false;
        }
        
        // Must have some alphabetic content
        if !trimmed.chars().any(|c| c.is_alphabetic()) {
            return false;
        }
        
        // Skip if looks like a path or identifier
        if PATH_OR_ID_RE.is_match(trimmed) {
            return false;
        }
        
        // Skip pure numbers with optional units
        if trimmed.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '%') {
            return false;
        }
        
        // Skip if it looks like code (contains operators or specific patterns)
        if trimmed.contains("==") || trimmed.contains("!=") || 
           trimmed.contains("&&") || trimmed.contains("||") ||
           trimmed.starts_with("li.") || trimmed.starts_with("def.") {
            return false;
        }
        
        // Skip color codes like "#FF0000"
        if trimmed.starts_with('#') && trimmed.len() <= 9 {
            return false;
        }
        
        true
    }
    
    /// Get the current tag context (simplified approach)
    fn get_tag_context(&self, content: &str, position: usize) -> Option<String> {
        // Look backwards for the opening tag
        let before = &content[..position];
        if let Some(tag_start) = before.rfind('<') {
            let tag_region = &before[tag_start..];
            // Extract tag name
            let tag_name: String = tag_region
                .chars()
                .skip(1)
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
                .collect();
            if !tag_name.is_empty() && !tag_name.starts_with('/') {
                return Some(tag_name);
            }
        }
        None
    }
    
    /// Check if position is inside a comment
    fn is_in_comment(&self, content: &str, position: usize) -> bool {
        for cap in COMMENT_RE.find_iter(content) {
            if cap.start() <= position && position < cap.end() {
                return true;
            }
        }
        false
    }
    
    /// Extract translatable attributes from a tag
    fn extract_attributes(&self, tag_content: &str, entries: &mut Vec<TranslatableEntry>, base_key: &str) {
        for cap in ATTR_PATTERN.captures_iter(tag_content) {
            let attr_name = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
            let attr_value = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
            
            // Check if this attribute should be translated
            let should_translate = self.translatable_attrs.iter()
                .any(|a| a.eq_ignore_ascii_case(attr_name));
            
            if should_translate && self.is_translatable_text(attr_value) {
                entries.push(TranslatableEntry {
                    key: format!("{}@{}", base_key, attr_name),
                    source: attr_value.to_string(),
                    context: Some(format!("attribute: {}", attr_name)),
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert("type".to_string(), "attribute".to_string());
                        meta.insert("attr_name".to_string(), attr_name.to_string());
                        meta
                    },
                });
            }
        }
    }
}

impl Default for XmlHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatHandler for XmlHandler {
    fn extract(&self, content: &str) -> Result<Vec<TranslatableEntry>, FormatError> {
        let mut entries = Vec::new();
        let mut seen_texts: HashMap<String, usize> = HashMap::new();
        
        // First, extract text content between tags
        for cap in TAG_CONTENT_RE.captures_iter(content) {
            if let Some(text_match) = cap.get(1) {
                let text = text_match.as_str().trim();
                let position = text_match.start();
                
                // Skip if inside a comment
                if self.is_in_comment(content, position) {
                    continue;
                }
                
                // Get tag context
                let tag = self.get_tag_context(content, position);
                
                // Skip if in a non-translatable tag
                if let Some(ref tag_name) = tag {
                    if self.skip_tags.iter().any(|t| t.eq_ignore_ascii_case(tag_name)) {
                        continue;
                    }
                }
                
                // Check if text is translatable
                if !self.is_translatable_text(text) {
                    continue;
                }
                
                // Generate unique key
                let base_key = tag.as_deref().unwrap_or("text");
                let occurrence = seen_texts.entry(text.to_string()).or_insert(0);
                *occurrence += 1;
                
                let key = if *occurrence > 1 {
                    format!("{}_{}_dup{}", base_key, position, occurrence)
                } else {
                    format!("{}_{}", base_key, position)
                };
                
                entries.push(TranslatableEntry {
                    key,
                    source: text.to_string(),
                    context: tag.map(|t| format!("<{}>", t)),
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert("type".to_string(), "element_text".to_string());
                        meta.insert("position".to_string(), position.to_string());
                        meta
                    },
                });
            }
        }
        
        // Extract CDATA content
        for (idx, cap) in CDATA_RE.captures_iter(content).enumerate() {
            if let Some(cdata_match) = cap.get(1) {
                let text = cdata_match.as_str().trim();
                if self.is_translatable_text(text) {
                    entries.push(TranslatableEntry {
                        key: format!("cdata_{}", idx),
                        source: text.to_string(),
                        context: Some("CDATA section".to_string()),
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("type".to_string(), "cdata".to_string());
                            meta
                        },
                    });
                }
            }
        }
        
        // Extract translatable attributes
        let tag_re = Regex::new(r"<([a-zA-Z_:][a-zA-Z0-9_:.-]*)\s+([^>]+)>")
            .map_err(|e| FormatError::ParseError(e.to_string()))?;
        
        for cap in tag_re.captures_iter(content) {
            let tag_name = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
            let attrs = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
            
            let base_key = format!("attr_{}", tag_name);
            self.extract_attributes(attrs, &mut entries, &base_key);
        }
        
        Ok(entries)
    }

    fn merge(
        &self,
        original: &str,
        translations: &TranslationResult,
    ) -> Result<String, FormatError> {
        let mut result = original.to_string();
        
        // Build a map of source -> target for efficient lookup
        let translation_map: HashMap<&str, &str> = translations.translated
            .iter()
            .map(|entry| (entry.source.as_str(), entry.target.as_str()))
            .collect();
        
        // Replace text content
        for cap in TAG_CONTENT_RE.captures_iter(original) {
            if let Some(text_match) = cap.get(1) {
                let text = text_match.as_str().trim();
                
                if let Some(&translation) = translation_map.get(text) {
                    // Replace while preserving surrounding whitespace
                    let full_match = cap.get(0).unwrap();
                    let original_with_tags = full_match.as_str();
                    
                    // Reconstruct with translated text
                    let new_content = original_with_tags.replace(text, translation);
                    result = result.replace(original_with_tags, &new_content);
                }
            }
        }
        
        // Replace CDATA content
        for cap in CDATA_RE.captures_iter(original) {
            if let Some(cdata_match) = cap.get(1) {
                let text = cdata_match.as_str().trim();
                
                if let Some(&translation) = translation_map.get(text) {
                    let full_match = cap.get(0).unwrap().as_str();
                    let new_cdata = full_match.replace(text, translation);
                    result = result.replace(full_match, &new_cdata);
                }
            }
        }
        
        // Replace attribute values
        for entry in &translations.translated {
            if entry.key.contains('@') {
                // This is an attribute translation
                // Find and replace the attribute value
                let source_escaped = regex::escape(&entry.source);
                let attr_pattern = format!(r#"(\w+\s*=\s*"){}"#, source_escaped);
                
                if let Ok(re) = Regex::new(&attr_pattern) {
                    result = re.replace_all(&result, |caps: &regex::Captures| {
                        format!("{}{}", &caps[1], &entry.target)
                    }).to_string();
                }
            }
        }
        
        Ok(result)
    }

    fn format(&self) -> FileFormat {
        FileFormat::Xml
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_xml() {
        let handler = XmlHandler::new();
        let xml = r#"<root>
            <title>Hello World</title>
            <description>This is a test</description>
        </root>"#;
        
        let entries = handler.extract(xml).unwrap();
        assert!(entries.len() >= 2);
        
        let sources: Vec<&str> = entries.iter().map(|e| e.source.as_str()).collect();
        assert!(sources.contains(&"Hello World"));
        assert!(sources.contains(&"This is a test"));
    }
    
    #[test]
    fn test_skip_code_content() {
        let handler = XmlHandler::new();
        let xml = r#"<root>
            <defName>Item_Sword</defName>
            <label>Iron Sword</label>
            <texPath>Things/Weapons/Sword</texPath>
        </root>"#;
        
        let entries = handler.extract(xml).unwrap();
        
        let sources: Vec<&str> = entries.iter().map(|e| e.source.as_str()).collect();
        // Should extract "Iron Sword" but not defName or texPath
        assert!(sources.contains(&"Iron Sword"));
        assert!(!sources.iter().any(|s| s.contains("Item_Sword")));
        assert!(!sources.iter().any(|s| s.contains("Things/Weapons")));
    }
    
    #[test]
    fn test_translatable_text_filter() {
        let handler = XmlHandler::new();
        
        // Should be translatable
        assert!(handler.is_translatable_text("Hello World"));
        assert!(handler.is_translatable_text("This is a test."));
        
        // Should NOT be translatable
        assert!(!handler.is_translatable_text("Item_Sword"));
        assert!(!handler.is_translatable_text("Things/Weapons/Sword"));
        assert!(!handler.is_translatable_text("12345"));
        assert!(!handler.is_translatable_text("#FF0000"));
        assert!(!handler.is_translatable_text(""));
    }
    
    #[test]
    fn test_extract_attributes() {
        let handler = XmlHandler::new();
        let xml = r#"<button label="Click me" title="Submit button" id="btn_submit" />"#;
        
        let entries = handler.extract(xml).unwrap();
        
        let sources: Vec<&str> = entries.iter().map(|e| e.source.as_str()).collect();
        assert!(sources.contains(&"Click me"));
        assert!(sources.contains(&"Submit button"));
        // id should not be extracted
        assert!(!sources.contains(&"btn_submit"));
    }
    
    #[test]
    fn test_merge_translations() {
        let handler = XmlHandler::new();
        let xml = r#"<root><title>Hello</title></root>"#;
        
        let translations = TranslationResult {
            translated: vec![
                super::super::TranslatedEntry {
                    key: "title_8".to_string(),
                    source: "Hello".to_string(),
                    target: "안녕하세요".to_string(),
                }
            ],
            failed: vec![],
        };
        
        let result = handler.merge(xml, &translations).unwrap();
        assert!(result.contains("안녕하세요"));
        assert!(!result.contains(">Hello<"));
    }
    
    #[test]
    fn test_skip_comments() {
        let handler = XmlHandler::new();
        let xml = r#"<root>
            <!-- This is a comment with text -->
            <title>Real Title</title>
        </root>"#;
        
        let entries = handler.extract(xml).unwrap();
        
        let sources: Vec<&str> = entries.iter().map(|e| e.source.as_str()).collect();
        assert!(sources.contains(&"Real Title"));
        assert!(!sources.iter().any(|s| s.contains("comment")));
    }
}
