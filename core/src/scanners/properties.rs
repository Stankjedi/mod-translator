/// Properties (Java .properties) scanner with token protection
/// 
/// Protects:
/// - Unicode escapes: \uXXXX
/// - Line continuations: backslash at end of line
/// - Format tokens: %s, %d, %1$s, {0}, {1:0.##}
/// - Keys and comments (non-translatable)
/// 
/// Translates: Value part of key=value pairs only

use regex::Regex;
use once_cell::sync::Lazy;
use std::collections::HashMap;

static UNICODE_ESCAPE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\\u[0-9a-fA-F]{4}")
        .expect("valid unicode escape regex")
});

static PRINTF_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"%(?:\d+\$)?[sdifcbxXeEgGaAn%]")
        .expect("valid printf regex")
});

static DOTNET_FORMAT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{(\d+)(?::[^}]+)?\}")
        .expect("valid dotnet format regex")
});

#[derive(Debug, Clone)]
pub struct PropertiesScanner {
    counter: usize,
}

impl PropertiesScanner {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    /// Scan properties text and protect tokens
    /// Returns the masked value part only (not including key)
    pub fn scan_value(&mut self, value: &str) -> ScanResult {
        let mut result = ScanResult::new();
        let mut current = value.to_string();
        
        // Phase 1: Protect unicode escapes
        current = self.protect_unicode_escapes(&current, &mut result);
        
        // Phase 2: Protect printf-style format tokens
        current = self.protect_printf_tokens(&current, &mut result);
        
        // Phase 3: Protect .NET-style format tokens
        current = self.protect_dotnet_tokens(&current, &mut result);
        
        // Phase 4: Protect line continuation markers
        current = self.protect_line_continuations(&current, &mut result);
        
        result.source_masked = current;
        result
    }

    /// Parse a properties file and return translatable entries
    pub fn parse_file(&self, content: &str) -> Vec<PropertiesEntry> {
        let mut entries = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                i += 1;
                continue;
            }
            
            // Parse key=value or key:value
            if let Some((key, mut value)) = Self::parse_key_value(line) {
                let line_num = i + 1;
                
                // Handle line continuations
                let mut full_value = value.to_string();
                while full_value.ends_with('\\') && i + 1 < lines.len() {
                    i += 1;
                    full_value.pop(); // Remove trailing backslash
                    full_value.push_str(lines[i].trim_start());
                }
                
                entries.push(PropertiesEntry {
                    key: key.to_string(),
                    value: full_value,
                    line: line_num,
                });
            }
            
            i += 1;
        }
        
        entries
    }

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

    fn protect_unicode_escapes(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        let matches: Vec<_> = UNICODE_ESCAPE_REGEX.find_iter(text).collect();
        
        for m in matches.iter().rev() {
            let token = self.next_token("ESCAPE");
            
            result.expected_tokens.push(token.clone());
            result.token_types.push("ESCAPE".to_string());
            *result.token_multiset.entry(token.clone()).or_insert(0) += 1;
            
            current.replace_range(m.range(), &token);
        }
        
        current
    }

    fn protect_printf_tokens(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        let matches: Vec<_> = PRINTF_REGEX.find_iter(text).collect();
        
        for m in matches.iter().rev() {
            // Skip %% (escaped percent)
            if m.as_str() == "%%" {
                continue;
            }
            
            let token = self.next_token("PRINTF");
            
            result.expected_tokens.push(token.clone());
            result.token_types.push("PRINTF".to_string());
            *result.token_multiset.entry(token.clone()).or_insert(0) += 1;
            
            current.replace_range(m.range(), &token);
        }
        
        current
    }

    fn protect_dotnet_tokens(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        let matches: Vec<_> = DOTNET_FORMAT_REGEX.find_iter(text).collect();
        
        for m in matches.iter().rev() {
            let token = self.next_token("DOTNET");
            
            result.expected_tokens.push(token.clone());
            result.token_types.push("DOTNET".to_string());
            *result.token_multiset.entry(token.clone()).or_insert(0) += 1;
            
            current.replace_range(m.range(), &token);
        }
        
        current
    }

    fn protect_line_continuations(&mut self, text: &str, result: &mut ScanResult) -> String {
        // Line continuations are handled at parse time, not in individual values
        // This is a no-op for individual value scanning
        text.to_string()
    }

    fn next_token(&mut self, token_type: &str) -> String {
        let token = format!("⟦MT:{}:{}⟧", token_type, self.counter);
        self.counter += 1;
        token
    }
}

#[derive(Debug, Clone)]
pub struct PropertiesEntry {
    pub key: String,
    pub value: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub source_masked: String,
    pub expected_tokens: Vec<String>,
    pub token_multiset: HashMap<String, usize>,
    pub token_types: Vec<String>,
}

impl ScanResult {
    fn new() -> Self {
        Self {
            source_masked: String::new(),
            expected_tokens: Vec::new(),
            token_multiset: HashMap::new(),
            token_types: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protect_unicode_escapes() {
        let mut scanner = PropertiesScanner::new();
        let text = "Hello \\u0048ello";
        let result = scanner.scan_value(text);
        
        assert!(result.source_masked.contains("⟦MT:ESCAPE:"));
        assert!(!result.source_masked.contains("\\u0048"));
        assert_eq!(result.expected_tokens.len(), 1);
    }

    #[test]
    fn test_protect_printf_tokens() {
        let mut scanner = PropertiesScanner::new();
        let text = "Player %s has %d points";
        let result = scanner.scan_value(text);
        
        assert!(result.source_masked.contains("⟦MT:PRINTF:"));
        assert_eq!(result.expected_tokens.len(), 2);
    }

    #[test]
    fn test_protect_dotnet_tokens() {
        let mut scanner = PropertiesScanner::new();
        let text = "Score: {0}, Time: {1:0.##}";
        let result = scanner.scan_value(text);
        
        assert!(result.source_masked.contains("⟦MT:DOTNET:"));
        assert_eq!(result.expected_tokens.len(), 2);
    }

    #[test]
    fn test_parse_key_value() {
        let scanner = PropertiesScanner::new();
        
        let (key, value) = PropertiesScanner::parse_key_value("message=Hello World").unwrap();
        assert_eq!(key, "message");
        assert_eq!(value, "Hello World");
        
        let (key, value) = PropertiesScanner::parse_key_value("key:value").unwrap();
        assert_eq!(key, "key");
        assert_eq!(value, "value");
    }

    #[test]
    fn test_parse_file() {
        let scanner = PropertiesScanner::new();
        let content = r#"
# Comment
message=Hello
count=You have %d items
unicode=\u0048ello
"#;
        let entries = scanner.parse_file(content);
        
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].key, "message");
        assert_eq!(entries[0].value, "Hello");
        assert_eq!(entries[1].key, "count");
        assert!(entries[1].value.contains("%d"));
    }

    #[test]
    fn test_line_continuation() {
        let scanner = PropertiesScanner::new();
        let content = "long.message=This is a \\\nvery long message";
        let entries = scanner.parse_file(content);
        
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "long.message");
        assert!(entries[0].value.contains("very long message"));
    }

    #[test]
    fn test_complex_value() {
        let mut scanner = PropertiesScanner::new();
        let text = "Player %s scored {0} points (\\u2605)";
        let result = scanner.scan_value(text);
        
        // Should protect %s, {0}, and \u2605
        assert!(result.expected_tokens.len() >= 3);
        assert!(result.source_masked.contains("Player"));
        assert!(result.source_masked.contains("scored"));
        assert!(result.source_masked.contains("points"));
    }
}
