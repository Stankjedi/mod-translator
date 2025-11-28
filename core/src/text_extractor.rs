/// Text Extractor Module
/// 
/// Extracts translatable text segments from various file formats.
/// This module is responsible for:
/// 1. Identifying which parts of a file should be translated
/// 2. Separating code/markup from natural language text
/// 3. Preserving context for accurate translation

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a segment of text that should be translated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatableSegment {
    /// Unique identifier for this segment
    pub id: String,
    /// The raw text to translate (may include protected tokens)
    pub text: String,
    /// Context: what comes before this segment
    pub prefix: String,
    /// Context: what comes after this segment
    pub suffix: String,
    /// Line number in the original file (1-indexed)
    pub line_number: usize,
    /// Column offset in the original line
    pub column: usize,
    /// The key/path to this element (for structured formats like XML/JSON)
    pub key: Option<String>,
    /// Whether this segment appears to be translatable
    pub is_translatable: bool,
    /// Reason if not translatable
    pub skip_reason: Option<String>,
}

/// Result of extracting translatable segments from a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Segments that should be translated
    pub translatable: Vec<TranslatableSegment>,
    /// Segments that were skipped (for reference)
    pub skipped: Vec<TranslatableSegment>,
    /// Detected file format
    pub format: DetectedFormat,
    /// Detected text style/tone hints
    pub style_hints: StyleHints,
}

/// Detected file format
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DetectedFormat {
    Xml,
    Json,
    Yaml,
    Ini,
    Properties,
    Lua,
    PlainText,
    Csv,
    Po,
    Unknown,
}

/// Style hints detected from the text
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleHints {
    /// Detected formality level (0.0 = very casual, 1.0 = very formal)
    pub formality: f32,
    /// Whether the text uses honorifics/politeness markers
    pub uses_honorifics: bool,
    /// Common terminology found
    pub terminology: HashMap<String, usize>,
    /// Whether the text appears to be game UI (short strings)
    pub is_ui_text: bool,
    /// Whether the text appears to be narrative/dialogue
    pub is_narrative: bool,
    /// Detected sentence endings style
    pub sentence_style: SentenceStyle,
}

/// Detected sentence ending style
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum SentenceStyle {
    #[default]
    Neutral,
    /// Formal/polite (e.g., Korean -습니다/-입니다)
    Formal,
    /// Casual/informal (e.g., Korean -어/-아)
    Casual,
    /// Mixed styles
    Mixed,
}

// Patterns for detecting non-translatable content
#[allow(dead_code)]
static CODE_BLOCK_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"```[\s\S]*?```|`[^`]+`").expect("valid code block pattern")
});

#[allow(dead_code)]
static COMMENT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<!--[\s\S]*?-->|/\*[\s\S]*?\*/|//[^\n]*|#[^\n]*").expect("valid comment pattern")
});

#[allow(dead_code)]
static CDATA_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<!\[CDATA\[[\s\S]*?\]\]>").expect("valid CDATA pattern")
});

static PATH_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[\w\-./\\]+$|^[A-Z]:\\|^/(?:usr|etc|var|home)/").expect("valid path pattern")
});

static VARIABLE_NAME_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").expect("valid variable pattern")
});

static NUMBER_ONLY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[\d.,\s\-%+*/=<>]+$").expect("valid number pattern")
});

static URL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https?://|^ftp://|^mailto:|^www\.").expect("valid url pattern")
});

/// Text Extractor - extracts translatable segments from various file formats
pub struct TextExtractor {
    /// Minimum word count for text to be considered translatable
    min_word_count: usize,
    /// Maximum ratio of non-letter characters
    max_special_char_ratio: f32,
}

impl Default for TextExtractor {
    fn default() -> Self {
        Self {
            min_word_count: 1,
            max_special_char_ratio: 0.7,
        }
    }
}

impl TextExtractor {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Check if a text segment should be translated
    pub fn is_translatable(&self, text: &str) -> (bool, Option<String>) {
        let trimmed = text.trim();
        
        // Empty or whitespace-only
        if trimmed.is_empty() {
            return (false, Some("Empty text".into()));
        }
        
        // Very short (single character or less)
        if trimmed.chars().count() < 2 {
            return (false, Some("Too short".into()));
        }
        
        // Pure numbers or mathematical expressions without text
        if NUMBER_ONLY_PATTERN.is_match(trimmed) {
            return (false, Some("Numbers only".into()));
        }
        
        // URLs
        if URL_PATTERN.is_match(trimmed) {
            return (false, Some("URL".into()));
        }
        
        // File paths
        if PATH_PATTERN.is_match(trimmed) && !trimmed.contains(' ') {
            return (false, Some("File path".into()));
        }
        
        // Variable/identifier names (no spaces, alphanumeric with underscores)
        if VARIABLE_NAME_PATTERN.is_match(trimmed) && trimmed.len() < 50 {
            // But allow if it looks like a word in natural language
            let has_vowels = trimmed.chars().any(|c| "aeiouAEIOU".contains(c));
            let has_mixed_case = trimmed.chars().any(|c| c.is_uppercase()) 
                && trimmed.chars().any(|c| c.is_lowercase());
            if !has_vowels || (!has_mixed_case && trimmed.chars().all(|c| c.is_uppercase() || c == '_')) {
                return (false, Some("Variable name".into()));
            }
        }
        
        // Count "word-like" characters (letters from any script)
        let letter_count = trimmed.chars().filter(|c| c.is_alphabetic()).count();
        let total_non_space = trimmed.chars().filter(|c| !c.is_whitespace()).count();
        
        if total_non_space > 0 {
            let letter_ratio = letter_count as f32 / total_non_space as f32;
            if letter_ratio < (1.0 - self.max_special_char_ratio) {
                return (false, Some(format!("Low letter ratio: {:.2}", letter_ratio)));
            }
        }
        
        // Count words (sequences of alphabetic characters or CJK characters)
        let word_count = count_words(trimmed);
        if word_count < self.min_word_count {
            return (false, Some(format!("Only {} word(s)", word_count)));
        }
        
        (true, None)
    }
    
    /// Extract translatable segments from XML content
    pub fn extract_xml(&self, content: &str) -> ExtractionResult {
        let mut translatable = Vec::new();
        let mut skipped = Vec::new();
        let style_hints = self.analyze_style(content);
        
        // Match text content between XML tags: >text<
        let tag_content_re = Regex::new(r">([^<]+)<").expect("valid regex");
        
        let mut segment_id = 0;
        for (line_num, line) in content.lines().enumerate() {
            for cap in tag_content_re.captures_iter(line) {
                if let Some(text_match) = cap.get(1) {
                    let text = text_match.as_str();
                    let trimmed = text.trim();
                    
                    if trimmed.is_empty() {
                        continue;
                    }
                    
                    let (is_trans, skip_reason) = self.is_translatable(trimmed);
                    
                    // Try to extract key from surrounding tags
                    let key = extract_xml_key(line, text_match.start());
                    
                    let segment = TranslatableSegment {
                        id: format!("xml_{}", segment_id),
                        text: trimmed.to_string(),
                        prefix: line[..text_match.start()].to_string(),
                        suffix: line[text_match.end()..].to_string(),
                        line_number: line_num + 1,
                        column: text_match.start(),
                        key,
                        is_translatable: is_trans,
                        skip_reason: skip_reason.clone(),
                    };
                    
                    if is_trans {
                        translatable.push(segment);
                    } else {
                        skipped.push(segment);
                    }
                    
                    segment_id += 1;
                }
            }
            
            // Also check for attribute values that might need translation
            // e.g., title="...", description="..."
            let attr_re = Regex::new(r#"(?:title|description|label|text|tooltip|hint|message|name)="([^"]+)""#).expect("valid regex");
            for cap in attr_re.captures_iter(line) {
                if let Some(text_match) = cap.get(1) {
                    let text = text_match.as_str();
                    let trimmed = text.trim();
                    
                    if trimmed.is_empty() {
                        continue;
                    }
                    
                    let (is_trans, skip_reason) = self.is_translatable(trimmed);
                    
                    let segment = TranslatableSegment {
                        id: format!("xml_attr_{}", segment_id),
                        text: trimmed.to_string(),
                        prefix: format!("{}=\"", &cap.get(0).unwrap().as_str().split('=').next().unwrap_or("")),
                        suffix: "\"".to_string(),
                        line_number: line_num + 1,
                        column: text_match.start(),
                        key: None,
                        is_translatable: is_trans,
                        skip_reason: skip_reason.clone(),
                    };
                    
                    if is_trans {
                        translatable.push(segment);
                    } else {
                        skipped.push(segment);
                    }
                    
                    segment_id += 1;
                }
            }
        }
        
        ExtractionResult {
            translatable,
            skipped,
            format: DetectedFormat::Xml,
            style_hints,
        }
    }
    
    /// Extract translatable segments from JSON content
    pub fn extract_json(&self, content: &str) -> ExtractionResult {
        let mut translatable = Vec::new();
        let mut skipped = Vec::new();
        let style_hints = self.analyze_style(content);
        
        // Match string values in JSON: "key": "value"
        let json_string_re = Regex::new(r#""([^"\\]*(?:\\.[^"\\]*)*)"\s*:\s*"([^"\\]*(?:\\.[^"\\]*)*)""#).expect("valid regex");
        
        let mut segment_id = 0;
        for (line_num, line) in content.lines().enumerate() {
            for cap in json_string_re.captures_iter(line) {
                if let (Some(key_match), Some(value_match)) = (cap.get(1), cap.get(2)) {
                    let key = key_match.as_str();
                    let value = value_match.as_str();
                    
                    // Skip if value is empty
                    if value.trim().is_empty() {
                        continue;
                    }
                    
                    let (is_trans, skip_reason) = self.is_translatable(value);
                    
                    let segment = TranslatableSegment {
                        id: format!("json_{}", segment_id),
                        text: value.to_string(),
                        prefix: format!("\"{}\": \"", key),
                        suffix: "\"".to_string(),
                        line_number: line_num + 1,
                        column: value_match.start(),
                        key: Some(key.to_string()),
                        is_translatable: is_trans,
                        skip_reason: skip_reason.clone(),
                    };
                    
                    if is_trans {
                        translatable.push(segment);
                    } else {
                        skipped.push(segment);
                    }
                    
                    segment_id += 1;
                }
            }
        }
        
        ExtractionResult {
            translatable,
            skipped,
            format: DetectedFormat::Json,
            style_hints,
        }
    }
    
    /// Extract translatable segments from INI/CFG content
    pub fn extract_ini(&self, content: &str) -> ExtractionResult {
        let mut translatable = Vec::new();
        let mut skipped = Vec::new();
        let style_hints = self.analyze_style(content);
        
        let mut segment_id = 0;
        let mut current_section = String::new();
        
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }
            
            // Section header
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = trimmed[1..trimmed.len()-1].to_string();
                continue;
            }
            
            // Key=Value pair
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim();
                let value = trimmed[eq_pos+1..].trim();
                
                if value.is_empty() {
                    continue;
                }
                
                let (is_trans, skip_reason) = self.is_translatable(value);
                
                let full_key = if current_section.is_empty() {
                    key.to_string()
                } else {
                    format!("{}/{}", current_section, key)
                };
                
                let segment = TranslatableSegment {
                    id: format!("ini_{}", segment_id),
                    text: value.to_string(),
                    prefix: format!("{}=", key),
                    suffix: String::new(),
                    line_number: line_num + 1,
                    column: eq_pos + 1,
                    key: Some(full_key),
                    is_translatable: is_trans,
                    skip_reason: skip_reason.clone(),
                };
                
                if is_trans {
                    translatable.push(segment);
                } else {
                    skipped.push(segment);
                }
                
                segment_id += 1;
            }
        }
        
        ExtractionResult {
            translatable,
            skipped,
            format: DetectedFormat::Ini,
            style_hints,
        }
    }
    
    /// Extract from plain text (line by line)
    pub fn extract_plain_text(&self, content: &str) -> ExtractionResult {
        let mut translatable = Vec::new();
        let mut skipped = Vec::new();
        let style_hints = self.analyze_style(content);
        
        let mut segment_id = 0;
        
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            
            if trimmed.is_empty() {
                continue;
            }
            
            let (is_trans, skip_reason) = self.is_translatable(trimmed);
            
            let segment = TranslatableSegment {
                id: format!("txt_{}", segment_id),
                text: trimmed.to_string(),
                prefix: String::new(),
                suffix: String::new(),
                line_number: line_num + 1,
                column: 0,
                key: None,
                is_translatable: is_trans,
                skip_reason: skip_reason.clone(),
            };
            
            if is_trans {
                translatable.push(segment);
            } else {
                skipped.push(segment);
            }
            
            segment_id += 1;
        }
        
        ExtractionResult {
            translatable,
            skipped,
            format: DetectedFormat::PlainText,
            style_hints,
        }
    }
    
    /// Analyze text style to provide hints for translation
    pub fn analyze_style(&self, content: &str) -> StyleHints {
        let mut hints = StyleHints::default();
        let mut terminology: HashMap<String, usize> = HashMap::new();
        
        // Analyze text characteristics
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        
        if total_lines == 0 {
            return hints;
        }
        
        let mut short_strings = 0;
        let mut question_marks = 0;
        let mut exclamation_marks = 0;
        let mut dialog_markers = 0;
        
        for line in &lines {
            let trimmed = line.trim();
            
            // Count short strings (UI-like)
            if trimmed.len() < 30 && !trimmed.is_empty() {
                short_strings += 1;
            }
            
            // Count punctuation for style detection
            question_marks += trimmed.matches('?').count();
            exclamation_marks += trimmed.matches('!').count();
            
            // Detect dialog markers
            if trimmed.starts_with('"') || trimmed.starts_with('\'') 
               || trimmed.contains("says") || trimmed.contains("said") {
                dialog_markers += 1;
            }
            
            // Collect potential terminology (capitalized words, repeated phrases)
            for word in trimmed.split_whitespace() {
                let cleaned: String = word.chars().filter(|c| c.is_alphabetic()).collect();
                if cleaned.len() >= 3 && cleaned.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    *terminology.entry(cleaned).or_insert(0) += 1;
                }
            }
        }
        
        // Calculate characteristics
        let short_ratio = short_strings as f32 / total_lines as f32;
        hints.is_ui_text = short_ratio > 0.7;
        hints.is_narrative = dialog_markers as f32 / total_lines as f32 > 0.3;
        
        // Formality detection (simplified)
        // Higher formality if less exclamation marks and more complex sentences
        let punct_ratio = (question_marks + exclamation_marks) as f32 / total_lines.max(1) as f32;
        hints.formality = (1.0 - punct_ratio.min(1.0)) * 0.5 + 0.5;
        
        // Filter terminology to only include words that appear multiple times
        hints.terminology = terminology.into_iter()
            .filter(|(_, count)| *count >= 2)
            .collect();
        
        // Detect sentence style (for Korean/Japanese targets)
        hints.sentence_style = SentenceStyle::Neutral;
        
        hints
    }
    
    /// Auto-detect format and extract
    pub fn extract_auto(&self, content: &str, filename: &str) -> ExtractionResult {
        let format = detect_format(content, filename);
        
        match format {
            DetectedFormat::Xml => self.extract_xml(content),
            DetectedFormat::Json => self.extract_json(content),
            DetectedFormat::Ini => self.extract_ini(content),
            _ => self.extract_plain_text(content),
        }
    }
}

/// Count words in text, supporting multiple scripts
fn count_words(text: &str) -> usize {
    let mut count = 0;
    let mut in_word = false;
    
    for c in text.chars() {
        let is_word_char = c.is_alphabetic() || 
            // CJK characters (each character is a "word")
            (c >= '\u{4E00}' && c <= '\u{9FFF}') ||
            (c >= '\u{3040}' && c <= '\u{309F}') || // Hiragana
            (c >= '\u{30A0}' && c <= '\u{30FF}') || // Katakana
            (c >= '\u{AC00}' && c <= '\u{D7AF}');   // Korean
        
        if is_word_char {
            if !in_word {
                count += 1;
                in_word = true;
            }
            // CJK characters: each character counts as a word
            if c >= '\u{4E00}' && c <= '\u{9FFF}' {
                count += 1;
            }
        } else {
            in_word = false;
        }
    }
    
    count
}

/// Detect file format from content and filename
fn detect_format(content: &str, filename: &str) -> DetectedFormat {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    
    match ext.as_str() {
        "xml" => DetectedFormat::Xml,
        "json" | "jsonl" => DetectedFormat::Json,
        "yaml" | "yml" => DetectedFormat::Yaml,
        "ini" | "cfg" => DetectedFormat::Ini,
        "properties" => DetectedFormat::Properties,
        "lua" => DetectedFormat::Lua,
        "csv" | "tsv" => DetectedFormat::Csv,
        "po" | "pot" => DetectedFormat::Po,
        "txt" | "md" | "markdown" => DetectedFormat::PlainText,
        _ => {
            // Try to detect from content
            let trimmed = content.trim();
            if trimmed.starts_with("<?xml") || trimmed.starts_with('<') {
                DetectedFormat::Xml
            } else if trimmed.starts_with('{') || trimmed.starts_with('[') {
                DetectedFormat::Json
            } else if trimmed.contains("msgid") && trimmed.contains("msgstr") {
                DetectedFormat::Po
            } else {
                DetectedFormat::Unknown
            }
        }
    }
}

/// Extract XML key from context
fn extract_xml_key(line: &str, position: usize) -> Option<String> {
    // Find the opening tag before the text position
    let before = &line[..position];
    if let Some(tag_start) = before.rfind('<') {
        let tag_content = &before[tag_start+1..];
        // Extract tag name (first word)
        let tag_name: String = tag_content
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.' || *c == ':')
            .collect();
        if !tag_name.is_empty() && !tag_name.starts_with('/') {
            return Some(tag_name);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_translatable_basic() {
        let extractor = TextExtractor::new();
        
        // Should be translatable
        assert!(extractor.is_translatable("Hello world").0);
        assert!(extractor.is_translatable("This is a test sentence.").0);
        assert!(extractor.is_translatable("안녕하세요 세계").0);
        
        // Should NOT be translatable
        assert!(!extractor.is_translatable("").0);
        assert!(!extractor.is_translatable("12345").0);
        assert!(!extractor.is_translatable("10-20").0);
        assert!(!extractor.is_translatable("some_variable_name").0);
        assert!(!extractor.is_translatable("/usr/bin/test").0);
        assert!(!extractor.is_translatable("https://example.com").0);
    }
    
    #[test]
    fn test_extract_xml() {
        let extractor = TextExtractor::new();
        let xml = r#"
        <root>
            <title>Hello World</title>
            <value>12345</value>
            <description>This is a description.</description>
        </root>
        "#;
        
        let result = extractor.extract_xml(xml);
        
        assert_eq!(result.format, DetectedFormat::Xml);
        assert!(result.translatable.len() >= 2);
        
        // Check that "Hello World" and description are translatable
        let texts: Vec<_> = result.translatable.iter().map(|s| s.text.as_str()).collect();
        assert!(texts.contains(&"Hello World"));
        assert!(texts.contains(&"This is a description."));
    }
    
    #[test]
    fn test_extract_json() {
        let extractor = TextExtractor::new();
        let json = r#"
        {
            "title": "Game Title",
            "version": "1.0.0",
            "description": "This is a game description."
        }
        "#;
        
        let result = extractor.extract_json(json);
        
        assert_eq!(result.format, DetectedFormat::Json);
        
        let texts: Vec<_> = result.translatable.iter().map(|s| s.text.as_str()).collect();
        assert!(texts.contains(&"Game Title"));
        assert!(texts.contains(&"This is a game description."));
    }
    
    #[test]
    fn test_extract_ini() {
        let extractor = TextExtractor::new();
        let ini = r#"
        [Section1]
        key1=Hello World
        key2=12345
        key3=This is a test.
        "#;
        
        let result = extractor.extract_ini(ini);
        
        assert_eq!(result.format, DetectedFormat::Ini);
        
        let texts: Vec<_> = result.translatable.iter().map(|s| s.text.as_str()).collect();
        assert!(texts.contains(&"Hello World"));
        assert!(texts.contains(&"This is a test."));
        
        // "12345" should be skipped
        let skipped_texts: Vec<_> = result.skipped.iter().map(|s| s.text.as_str()).collect();
        assert!(skipped_texts.contains(&"12345"));
    }
    
    #[test]
    fn test_style_analysis() {
        let extractor = TextExtractor::new();
        
        // UI-like content (short strings)
        let ui_content = "Start\nOptions\nQuit\nSave\nLoad";
        let hints = extractor.analyze_style(ui_content);
        assert!(hints.is_ui_text);
        
        // Narrative content
        let narrative = r#"
        "Hello there!" he said excitedly.
        "How are you doing?" she replied.
        The sun was setting in the distance.
        "#;
        let hints = extractor.analyze_style(narrative);
        assert!(hints.is_narrative);
    }
    
    #[test]
    fn test_count_words() {
        assert_eq!(count_words("Hello world"), 2);
        assert_eq!(count_words("This is a test"), 4);
        // Korean text counts as 1 word (no whitespace separators)
        assert_eq!(count_words("안녕하세요"), 1);
        // Mixed: "Hello" = 1 word, "세계" = 1 word
        assert_eq!(count_words("Hello 세계"), 2);
    }
}
