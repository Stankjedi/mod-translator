/// Markdown scanner with token protection
/// 
/// Protects:
/// - Code spans: `code`
/// - Code blocks: ```lang\ncode\n```
/// - Links: [text](url "title")
/// - Images: ![alt](src)
/// - Inline math: $...$, \(...\)
/// - Display math: \[...\]
/// - HTML tags: <tag>
/// - Reference links: [label]: url "title"
/// 
/// Translates: Natural text in paragraphs, headers, lists

use regex::Regex;
use once_cell::sync::Lazy;
use std::collections::HashMap;

static CODE_FENCE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^```[\w]*\n(?s:.*?)^```$|^~~~[\w]*\n(?s:.*?)^~~~$")
        .expect("valid code fence regex")
});

static CODE_SPAN_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"`[^`]+`")
        .expect("valid code span regex")
});

static LINK_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[([^\]]+)\]\(([^)]+)\)")
        .expect("valid link regex")
});

static IMAGE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)")
        .expect("valid image regex")
});

static INLINE_MATH_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\$[^$]+\$|\\\\?\([^)]+\\\\?\)")
        .expect("valid inline math regex")
});

static DISPLAY_MATH_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\\\\?\[[^\]]+\\\\?\]")
        .expect("valid display math regex")
});

static HTML_TAG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"</?[\w]+[^>]*>")
        .expect("valid html tag regex")
});

static REF_LINK_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^\[([^\]]+)\]:\s*(.+)$")
        .expect("valid reference link regex")
});

#[derive(Debug, Clone)]
pub struct MarkdownScanner {
    counter: usize,
}

impl MarkdownScanner {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    /// Scan markdown text and protect tokens
    pub fn scan(&mut self, text: &str) -> ScanResult {
        let mut result = ScanResult::new();
        let mut current = text.to_string();
        
        // Phase 1: Protect code fences (outermost)
        current = self.protect_code_fences(&current, &mut result);
        
        // Phase 2: Protect inline code spans
        current = self.protect_code_spans(&current, &mut result);
        
        // Phase 3: Protect display math (before inline math)
        current = self.protect_display_math(&current, &mut result);
        
        // Phase 4: Protect inline math
        current = self.protect_inline_math(&current, &mut result);
        
        // Phase 5: Protect reference links
        current = self.protect_ref_links(&current, &mut result);
        
        // Phase 6: Protect images (before regular links)
        current = self.protect_images(&current, &mut result);
        
        // Phase 7: Protect regular links
        current = self.protect_links(&current, &mut result);
        
        // Phase 8: Protect HTML tags
        current = self.protect_html_tags(&current, &mut result);
        
        result.source_masked = current;
        result
    }

    fn protect_code_fences(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        
        // Find all code fence matches
        let matches: Vec<_> = CODE_FENCE_REGEX.find_iter(text).collect();
        
        // Process in reverse to maintain string positions
        for m in matches.iter().rev() {
            let original = m.as_str();
            let token = self.next_token("CODE_FENCE");
            
            result.expected_tokens.push(token.clone());
            result.token_types.push("CODE_FENCE".to_string());
            *result.token_multiset.entry(token.clone()).or_insert(0) += 1;
            
            current.replace_range(m.range(), &token);
        }
        
        current
    }

    fn protect_code_spans(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        let matches: Vec<_> = CODE_SPAN_REGEX.find_iter(text).collect();
        
        for m in matches.iter().rev() {
            let token = self.next_token("CODE_SPAN");
            
            result.expected_tokens.push(token.clone());
            result.token_types.push("CODE_SPAN".to_string());
            *result.token_multiset.entry(token.clone()).or_insert(0) += 1;
            
            current.replace_range(m.range(), &token);
        }
        
        current
    }

    fn protect_display_math(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        let matches: Vec<_> = DISPLAY_MATH_REGEX.find_iter(text).collect();
        
        for m in matches.iter().rev() {
            let token = self.next_token("MATHEXPR");
            
            result.expected_tokens.push(token.clone());
            result.token_types.push("MATHEXPR".to_string());
            *result.token_multiset.entry(token.clone()).or_insert(0) += 1;
            
            current.replace_range(m.range(), &token);
        }
        
        current
    }

    fn protect_inline_math(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        let matches: Vec<_> = INLINE_MATH_REGEX.find_iter(text).collect();
        
        for m in matches.iter().rev() {
            let token = self.next_token("MATHEXPR");
            
            result.expected_tokens.push(token.clone());
            result.token_types.push("MATHEXPR".to_string());
            *result.token_multiset.entry(token.clone()).or_insert(0) += 1;
            
            current.replace_range(m.range(), &token);
        }
        
        current
    }

    fn protect_ref_links(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        let matches: Vec<_> = REF_LINK_REGEX.find_iter(text).collect();
        
        for m in matches.iter().rev() {
            let token = self.next_token("MARKDOWN_REF");
            
            result.expected_tokens.push(token.clone());
            result.token_types.push("MARKDOWN_REF".to_string());
            *result.token_multiset.entry(token.clone()).or_insert(0) += 1;
            
            current.replace_range(m.range(), &token);
        }
        
        current
    }

    fn protect_images(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        
        // Collect all matches first
        let matches: Vec<_> = IMAGE_REGEX.find_iter(text).collect();
        
        // Process in reverse to maintain string positions
        for m in matches.iter().rev() {
            // Replace entire image syntax with a token
            let token = self.next_token("MARKDOWN_IMAGE");
            result.expected_tokens.push(token.clone());
            result.token_types.push("MARKDOWN_IMAGE".to_string());
            *result.token_multiset.entry(token.clone()).or_insert(0) += 1;
            
            current.replace_range(m.range(), &token);
        }
        
        current
    }

    fn protect_links(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        
        // Collect all matches first
        let matches: Vec<_> = LINK_REGEX.find_iter(text).collect();
        
        // Process in reverse to maintain string positions
        for m in matches.iter().rev() {
            // Replace entire link syntax with a token
            let token = self.next_token("MARKDOWN_LINK");
            result.expected_tokens.push(token.clone());
            result.token_types.push("MARKDOWN_LINK".to_string());
            *result.token_multiset.entry(token.clone()).or_insert(0) += 1;
            
            current.replace_range(m.range(), &token);
        }
        
        current
    }

    fn protect_html_tags(&mut self, text: &str, result: &mut ScanResult) -> String {
        let mut current = text.to_string();
        let matches: Vec<_> = HTML_TAG_REGEX.find_iter(text).collect();
        
        for m in matches.iter().rev() {
            let token = self.next_token("TAG");
            
            result.expected_tokens.push(token.clone());
            result.token_types.push("TAG".to_string());
            *result.token_multiset.entry(token.clone()).or_insert(0) += 1;
            
            current.replace_range(m.range(), &token);
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
    fn test_protect_code_spans() {
        let mut scanner = MarkdownScanner::new();
        let text = "Use the `println!` macro to print.";
        let result = scanner.scan(text);
        
        assert!(result.source_masked.contains("⟦MT:CODE_SPAN:"));
        assert!(!result.source_masked.contains("println!"));
        assert_eq!(result.expected_tokens.len(), 1);
    }

    #[test]
    fn test_protect_code_fences() {
        let mut scanner = MarkdownScanner::new();
        let text = "Example:\n```rust\nfn main() {}\n```\nEnd.";
        let result = scanner.scan(text);
        
        assert!(result.source_masked.contains("⟦MT:CODE_FENCE:"));
        assert!(!result.source_masked.contains("fn main()"));
    }

    #[test]
    fn test_protect_links() {
        let mut scanner = MarkdownScanner::new();
        let text = "Check [documentation](https://example.com) for details.";
        let result = scanner.scan(text);
        
        assert!(result.source_masked.contains("⟦MT:MARKDOWN_LINK:"));
        assert!(!result.source_masked.contains("documentation"));
        assert!(!result.source_masked.contains("https://example.com"));
    }

    #[test]
    fn test_protect_images() {
        let mut scanner = MarkdownScanner::new();
        let text = "Screenshot: ![Example screenshot](image.png)";
        let result = scanner.scan(text);
        
        eprintln!("Original: {}", text);
        eprintln!("Masked: {}", result.source_masked);
        eprintln!("Tokens: {:?}", result.expected_tokens);
        
        assert!(result.source_masked.contains("⟦MT:MARKDOWN_IMAGE:"));
        assert!(!result.source_masked.contains("Example screenshot"));
        assert!(!result.source_masked.contains("image.png"));
    }

    #[test]
    fn test_protect_inline_math() {
        let mut scanner = MarkdownScanner::new();
        let text = "The formula $E = mc^2$ is famous.";
        let result = scanner.scan(text);
        
        assert!(result.source_masked.contains("⟦MT:MATHEXPR:"));
        assert!(!result.source_masked.contains("E = mc^2"));
    }

    #[test]
    fn test_complex_markdown() {
        let mut scanner = MarkdownScanner::new();
        let text = r#"# Header

Use `code` and [link](url).

```rust
fn test() {}
```

Math: $x = y$
"#;
        let result = scanner.scan(text);
        
        // Should have multiple protected tokens
        assert!(result.expected_tokens.len() >= 4);
        assert!(result.source_masked.contains("Header"));
        assert!(!result.source_masked.contains("fn test()"));
    }
}
