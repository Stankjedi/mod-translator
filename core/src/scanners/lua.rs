/// Lua scanner with string literal token protection
/// 
/// Protects:
/// - String literal boundaries and escapes: ", ', \", \\, \n
/// - Format tokens: %s, %d, {0}, ICU blocks
/// - String concatenation operators: ..
/// 
/// Translates: String literal values only
/// Non-translatable: Keys, function/variable names, comments

use regex::Regex;
use once_cell::sync::Lazy;
use std::collections::HashMap;

static PRINTF_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"%(?:\d+\$)?[sdifcbxXeEgGaAn%]")
        .expect("valid printf regex")
});

static DOTNET_FORMAT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{(\d+)(?::[^}]+)?\}")
        .expect("valid dotnet format regex")
});

static ICU_BLOCK_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{[^}]*(?:plural|select|selectordinal)[^}]*\}")
        .expect("valid icu block regex")
});

#[derive(Debug, Clone)]
pub struct LuaScanner {
    counter: usize,
}

impl LuaScanner {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    /// Parse Lua file and extract string literals
    pub fn parse_file(&self, content: &str) -> Vec<LuaStringLiteral> {
        let mut literals = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        let mut i = 0;
        let mut line = 1;
        
        while i < chars.len() {
            // Skip comments
            if i + 1 < chars.len() && chars[i] == '-' && chars[i + 1] == '-' {
                // Check for block comment
                if i + 3 < chars.len() && chars[i + 2] == '[' && chars[i + 3] == '[' {
                    // Block comment, skip until ]]
                    i += 4;
                    while i + 1 < chars.len() {
                        if chars[i] == '\n' {
                            line += 1;
                        }
                        if chars[i] == ']' && chars[i + 1] == ']' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                } else {
                    // Line comment, skip until newline
                    while i < chars.len() && chars[i] != '\n' {
                        i += 1;
                    }
                    if i < chars.len() && chars[i] == '\n' {
                        line += 1;
                        i += 1;
                    }
                }
                continue;
            }
            
            // Check for string literals
            if chars[i] == '"' || chars[i] == '\'' {
                let quote_char = chars[i];
                let _start = i;
                let start_line = line;
                i += 1;
                
                let mut string_content = String::new();
                let mut escaped = false;
                
                while i < chars.len() {
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    
                    if escaped {
                        string_content.push(chars[i]);
                        escaped = false;
                        i += 1;
                        continue;
                    }
                    
                    if chars[i] == '\\' {
                        escaped = true;
                        string_content.push(chars[i]);
                        i += 1;
                        continue;
                    }
                    
                    if chars[i] == quote_char {
                        i += 1;
                        break;
                    }
                    
                    string_content.push(chars[i]);
                    i += 1;
                }
                
                literals.push(LuaStringLiteral {
                    content: string_content,
                    line: start_line,
                    quote_type: if quote_char == '"' { QuoteType::Double } else { QuoteType::Single },
                });
                continue;
            }
            
            // Check for long strings [[...]]
            if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '[' {
                let start_line = line;
                i += 2;
                
                let mut string_content = String::new();
                
                while i + 1 < chars.len() {
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    
                    if chars[i] == ']' && chars[i + 1] == ']' {
                        i += 2;
                        break;
                    }
                    
                    string_content.push(chars[i]);
                    i += 1;
                }
                
                literals.push(LuaStringLiteral {
                    content: string_content,
                    line: start_line,
                    quote_type: QuoteType::Long,
                });
                continue;
            }
            
            if chars[i] == '\n' {
                line += 1;
            }
            i += 1;
        }
        
        literals
    }

    /// Scan a string literal value and protect tokens
    pub fn scan_string(&mut self, content: &str) -> ScanResult {
        let mut result = ScanResult::new();
        let mut current = content.to_string();
        
        // Phase 1: Protect ICU blocks
        current = self.protect_icu_blocks(&current, &mut result);
        
        // Phase 2: Protect printf-style format tokens
        current = self.protect_printf_tokens(&current, &mut result);
        
        // Phase 3: Protect .NET-style format tokens
        current = self.protect_dotnet_tokens(&current, &mut result);
        
        // Phase 4: Protect escape sequences
        current = self.protect_escapes(&current, &mut result);
        
        result.source_masked = current;
        result
    }

    fn protect_icu_blocks(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        let matches: Vec<_> = ICU_BLOCK_REGEX.find_iter(text).collect();
        
        for m in matches.iter().rev() {
            let token = self.next_token("ICU");
            
            result.expected_tokens.push(token.clone());
            result.token_types.push("ICU".to_string());
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

    fn protect_escapes(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        let mut replacements: Vec<(usize, usize, String)> = Vec::new();
        
        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() {
                // Found escape sequence
                let start = i;
                let end = i + 2;
                
                let token = self.next_token("ESCAPE");
                result.expected_tokens.push(token.clone());
                result.token_types.push("ESCAPE".to_string());
                *result.token_multiset.entry(token.clone()).or_insert(0) += 1;
                
                replacements.push((start, end, token));
                i += 2;
            } else {
                i += 1;
            }
        }
        
        // Apply replacements in reverse order
        for (start, end, token) in replacements.iter().rev() {
            current.replace_range(*start..*end, token);
        }
        
        current
    }

    fn next_token(&mut self, token_type: &str) -> String {
        let token = format!("⟦MT:{}:{}⟧", token_type, self.counter);
        self.counter += 1;
        token
    }
}

#[derive(Debug, Clone)]
pub struct LuaStringLiteral {
    pub content: String,
    pub line: usize,
    pub quote_type: QuoteType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteType {
    Single,
    Double,
    Long,
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
    fn test_parse_double_quote_strings() {
        let scanner = LuaScanner::new();
        let content = r#"local msg = "Hello World""#;
        let literals = scanner.parse_file(content);
        
        assert_eq!(literals.len(), 1);
        assert_eq!(literals[0].content, "Hello World");
        assert_eq!(literals[0].quote_type, QuoteType::Double);
    }

    #[test]
    fn test_parse_single_quote_strings() {
        let scanner = LuaScanner::new();
        let content = r#"local msg = 'Hello World'"#;
        let literals = scanner.parse_file(content);
        
        assert_eq!(literals.len(), 1);
        assert_eq!(literals[0].content, "Hello World");
        assert_eq!(literals[0].quote_type, QuoteType::Single);
    }

    #[test]
    fn test_parse_long_strings() {
        let scanner = LuaScanner::new();
        let content = r#"local msg = [[Hello
World]]"#;
        let literals = scanner.parse_file(content);
        
        assert_eq!(literals.len(), 1);
        assert!(literals[0].content.contains("Hello"));
        assert_eq!(literals[0].quote_type, QuoteType::Long);
    }

    #[test]
    fn test_skip_comments() {
        let scanner = LuaScanner::new();
        let content = r#"
-- This is a comment with "string"
local msg = "Real string"
--[[ Block comment
with "string" inside
]]
"#;
        let literals = scanner.parse_file(content);
        
        assert_eq!(literals.len(), 1);
        assert_eq!(literals[0].content, "Real string");
    }

    #[test]
    fn test_protect_printf_tokens() {
        let mut scanner = LuaScanner::new();
        let text = "Player %s has %d points";
        let result = scanner.scan_string(text);
        
        assert!(result.source_masked.contains("⟦MT:PRINTF:"));
        assert_eq!(result.expected_tokens.len(), 2);
    }

    #[test]
    fn test_protect_escapes() {
        let mut scanner = LuaScanner::new();
        let text = r#"Line 1\nLine 2\"Quoted\""#;
        let result = scanner.scan_string(text);
        
        assert!(result.source_masked.contains("⟦MT:ESCAPE:"));
        assert!(result.expected_tokens.len() >= 3); // \n, \", \"
    }

    #[test]
    fn test_protect_dotnet_tokens() {
        let mut scanner = LuaScanner::new();
        let text = "Score: {0}, Time: {1:0.##}";
        let result = scanner.scan_string(text);
        
        assert!(result.source_masked.contains("⟦MT:DOTNET:"));
        assert_eq!(result.expected_tokens.len(), 2);
    }

    #[test]
    fn test_complex_lua_file() {
        let scanner = LuaScanner::new();
        let content = r#"
-- Localization table
local L = {
    greeting = "Hello %s!",
    farewell = 'Goodbye',
    multiline = [[This is
    a long string]],
    -- comment
    formatted = "Score: {0}"
}
return L
"#;
        let literals = scanner.parse_file(content);
        
        assert_eq!(literals.len(), 4);
        assert!(literals[0].content.contains("%s"));
    }
}
