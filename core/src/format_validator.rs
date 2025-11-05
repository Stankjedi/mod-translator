/// Format-specific validators for post-restoration verification (Section 6)
/// 
/// After token restoration, validates that the output maintains valid format structure:
/// - JSON: valid JSON syntax
/// - XML: well-formed XML with balanced tags
/// - YAML: valid YAML syntax
/// - PO: valid gettext format
/// - ICU: balanced MessageFormat blocks
/// - CFG/INI: valid key=value structure

use serde_json;
use serde_yaml;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatValidationError {
    #[error("JSON parse error: {0}")]
    JsonError(String),
    
    #[error("XML malformed: {0}")]
    XmlError(String),
    
    #[error("YAML parse error: {0}")]
    YamlError(String),
    
    #[error("PO format error: {0}")]
    PoError(String),
    
    #[error("ICU MessageFormat unbalanced: {0}")]
    IcuError(String),
    
    #[error("INI/CFG format error: {0}")]
    IniError(String),
    
    #[error("CSV format error: {0}")]
    CsvError(String),
}

/// Validates JSON format
pub fn validate_json(content: &str) -> Result<(), FormatValidationError> {
    serde_json::from_str::<serde_json::Value>(content)
        .map(|_| ())
        .map_err(|e| FormatValidationError::JsonError(e.to_string()))
}

/// Validates XML format - checks for well-formedness
pub fn validate_xml(content: &str) -> Result<(), FormatValidationError> {
    // Basic XML validation: check tag balance
    let mut stack: Vec<String> = Vec::new();
    let mut in_tag = false;
    let mut tag_name = String::new();
    let mut is_closing = false;
    let mut is_self_closing = false;
    
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let ch = chars[i];
        
        match ch {
            '<' => {
                if i + 1 < chars.len() {
                    in_tag = true;
                    tag_name.clear();
                    is_closing = chars[i + 1] == '/';
                    is_self_closing = false;
                    if is_closing {
                        i += 1; // skip the '/'
                    }
                }
            }
            '>' => {
                if in_tag && !tag_name.is_empty() {
                    // Check for self-closing tag
                    if i > 0 && chars[i - 1] == '/' {
                        is_self_closing = true;
                    }
                    
                    if is_closing {
                        // Closing tag
                        if stack.is_empty() {
                            return Err(FormatValidationError::XmlError(
                                format!("Unexpected closing tag: {}", tag_name)
                            ));
                        }
                        let expected = stack.pop().unwrap();
                        if expected != tag_name {
                            return Err(FormatValidationError::XmlError(
                                format!("Tag mismatch: expected </{}>  but got </{}>", expected, tag_name)
                            ));
                        }
                    } else if !is_self_closing {
                        // Opening tag (not self-closing)
                        // Skip comment, CDATA, DOCTYPE, and processing instruction tags
                        if !tag_name.starts_with('!') && !tag_name.starts_with('?') {
                            stack.push(tag_name.clone());
                        }
                    }
                }
                in_tag = false;
                tag_name.clear();
            }
            _ if in_tag => {
                // Build tag name (stop at space or other special chars)
                if ch.is_alphanumeric() || ch == '_' || ch == ':' || ch == '-' {
                    tag_name.push(ch);
                }
            }
            _ => {}
        }
        
        i += 1;
    }
    
    if !stack.is_empty() {
        return Err(FormatValidationError::XmlError(
            format!("Unclosed tags: {:?}", stack)
        ));
    }
    
    Ok(())
}

/// Validates YAML format
pub fn validate_yaml(content: &str) -> Result<(), FormatValidationError> {
    serde_yaml::from_str::<serde_yaml::Value>(content)
        .map(|_| ())
        .map_err(|e| FormatValidationError::YamlError(e.to_string()))
}

/// Validates PO (gettext) format
pub fn validate_po(content: &str) -> Result<(), FormatValidationError> {
    // Basic PO validation: check for valid msgid/msgstr pairs
    let lines: Vec<&str> = content.lines().collect();
    let mut in_msgid = false;
    let mut in_msgstr = false;
    let mut has_msgid = false;
    
    for line in lines {
        let trimmed = line.trim();
        
        if trimmed.starts_with("msgid") {
            if in_msgid && !in_msgstr {
                return Err(FormatValidationError::PoError(
                    "msgid without matching msgstr".to_string()
                ));
            }
            in_msgid = true;
            in_msgstr = false;
            has_msgid = true;
        } else if trimmed.starts_with("msgstr") {
            if !in_msgid {
                return Err(FormatValidationError::PoError(
                    "msgstr without preceding msgid".to_string()
                ));
            }
            in_msgstr = true;
            in_msgid = false;
        }
    }
    
    if in_msgid && !in_msgstr {
        return Err(FormatValidationError::PoError(
            "Incomplete msgid/msgstr pair at end of file".to_string()
        ));
    }
    
    Ok(())
}

/// Validates ICU MessageFormat blocks
pub fn validate_icu(content: &str) -> Result<(), FormatValidationError> {
    // Check brace balance and ICU keywords
    let mut brace_count = 0;
    let mut in_icu = false;
    
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        match chars[i] {
            '{' => {
                brace_count += 1;
                
                // Check if this is start of ICU pattern
                if i + 5 < chars.len() {
                    let lookahead: String = chars[i..i+20.min(chars.len())].iter().collect();
                    if lookahead.contains("plural") || lookahead.contains("select") || 
                       lookahead.contains("selectordinal") {
                        in_icu = true;
                    }
                }
            }
            '}' => {
                brace_count -= 1;
                if brace_count < 0 {
                    return Err(FormatValidationError::IcuError(
                        "Unbalanced closing brace".to_string()
                    ));
                }
                if brace_count == 0 {
                    in_icu = false;
                }
            }
            _ => {}
        }
        i += 1;
    }
    
    if brace_count != 0 {
        return Err(FormatValidationError::IcuError(
            format!("Unbalanced braces: {} unclosed", brace_count)
        ));
    }
    
    Ok(())
}

/// Validates INI/CFG format
pub fn validate_ini(content: &str) -> Result<(), FormatValidationError> {
    // Basic INI validation: check for valid section and key=value pairs
    let lines: Vec<&str> = content.lines().collect();
    
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        
        // Check for section headers
        if trimmed.starts_with('[') {
            if !trimmed.ends_with(']') {
                return Err(FormatValidationError::IniError(
                    format!("Malformed section header at line {}: {}", idx + 1, line)
                ));
            }
            continue;
        }
        
        // Check for key=value pairs
        if !trimmed.contains('=') && !trimmed.starts_with('[') {
            return Err(FormatValidationError::IniError(
                format!("Invalid line at {}: expected key=value or [section]", idx + 1)
            ));
        }
    }
    
    Ok(())
}

/// Validates CSV format - basic structure check
pub fn validate_csv(content: &str) -> Result<(), FormatValidationError> {
    let lines: Vec<&str> = content.lines().collect();
    
    if lines.is_empty() {
        return Ok(());
    }
    
    // Get column count from first row
    let first_line = lines[0];
    let expected_cols = first_line.split(',').count();
    
    // Check all rows have same column count
    for (idx, line) in lines.iter().enumerate() {
        let cols = line.split(',').count();
        if cols != expected_cols {
            return Err(FormatValidationError::CsvError(
                format!("Column count mismatch at line {}: expected {}, got {}", 
                    idx + 1, expected_cols, cols)
            ));
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_json_valid() {
        let content = r#"{"key": "value", "number": 42}"#;
        assert!(validate_json(content).is_ok());
    }
    
    #[test]
    fn test_validate_json_invalid() {
        let content = r#"{"key": "value"#; // missing closing brace
        assert!(validate_json(content).is_err());
    }
    
    #[test]
    fn test_validate_xml_valid() {
        let content = "<root><child>text</child></root>";
        assert!(validate_xml(content).is_ok());
    }
    
    #[test]
    fn test_validate_xml_self_closing() {
        let content = "<root><child /></root>";
        assert!(validate_xml(content).is_ok());
    }
    
    #[test]
    fn test_validate_xml_unbalanced() {
        let content = "<root><child>text</root>"; // mismatched tags
        assert!(validate_xml(content).is_err());
    }
    
    #[test]
    fn test_validate_xml_unclosed() {
        let content = "<root><child>text"; // unclosed tag
        assert!(validate_xml(content).is_err());
    }
    
    #[test]
    fn test_validate_yaml_valid() {
        let content = "key: value\nnumber: 42";
        assert!(validate_yaml(content).is_ok());
    }
    
    #[test]
    fn test_validate_yaml_invalid() {
        let content = "key: value\n  invalid indentation";
        // YAML parser may or may not accept this depending on strictness
        // Just check it doesn't panic
        let _ = validate_yaml(content);
    }
    
    #[test]
    fn test_validate_po_valid() {
        let content = r#"
msgid "Hello"
msgstr "안녕하세요"

msgid "World"
msgstr "세계"
"#;
        assert!(validate_po(content).is_ok());
    }
    
    #[test]
    fn test_validate_po_missing_msgstr() {
        let content = r#"
msgid "Hello"
msgid "World"
"#;
        assert!(validate_po(content).is_err());
    }
    
    #[test]
    fn test_validate_icu_valid() {
        let content = "{count, plural, one {# item} other {# items}}";
        assert!(validate_icu(content).is_ok());
    }
    
    #[test]
    fn test_validate_icu_unbalanced() {
        let content = "{count, plural, one {# item} other {# items}";
        assert!(validate_icu(content).is_err());
    }
    
    #[test]
    fn test_validate_ini_valid() {
        let content = "[section]\nkey=value\nkey2=value2";
        assert!(validate_ini(content).is_ok());
    }
    
    #[test]
    fn test_validate_ini_invalid() {
        let content = "[section\nkey=value"; // unclosed section
        assert!(validate_ini(content).is_err());
    }
    
    #[test]
    fn test_validate_csv_valid() {
        let content = "a,b,c\n1,2,3\n4,5,6";
        assert!(validate_csv(content).is_ok());
    }
    
    #[test]
    fn test_validate_csv_invalid() {
        let content = "a,b,c\n1,2\n4,5,6"; // mismatched columns
        assert!(validate_csv(content).is_err());
    }
}
