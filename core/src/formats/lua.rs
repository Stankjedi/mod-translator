/// Lua format handler with scanner integration
use super::{FileFormat, FormatError, FormatHandler, TranslatableEntry, TranslationResult};
use crate::scanners::LuaScanner;
use std::collections::HashMap;

pub struct LuaHandler {
    scanner: LuaScanner,
}

impl LuaHandler {
    pub fn new() -> Self {
        Self {
            scanner: LuaScanner::new(),
        }
    }
}

impl FormatHandler for LuaHandler {
    fn extract(&self, content: &str) -> Result<Vec<TranslatableEntry>, FormatError> {
        let literals = self.scanner.parse_file(content);
        
        let mut translatable = Vec::new();
        
        for (idx, literal) in literals.iter().enumerate() {
            let mut metadata = HashMap::new();
            metadata.insert("line".to_string(), literal.line.to_string());
            metadata.insert("quote_type".to_string(), format!("{:?}", literal.quote_type));
            
            translatable.push(TranslatableEntry {
                key: format!("string_{}", idx),
                source: literal.content.clone(),
                context: Some(format!("line {}", literal.line)),
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
        // Parse original to get string positions
        let _literals = self.scanner.parse_file(original);
        
        // Build translation map by index
        let mut translation_map: HashMap<String, String> = HashMap::new();
        for t in &translations.translated {
            translation_map.insert(t.key.clone(), t.target.clone());
        }
        
        // Since replacing strings in Lua requires careful position tracking,
        // and the string positions can shift, we'll need to process from end to start
        let result = original.to_string();
        let _chars: Vec<char> = original.chars().collect();
        
        // For simplicity in this initial implementation, we'll just return the original
        // A full implementation would need to track exact positions and do careful replacement
        // This is a complex task that requires maintaining position information during parsing
        
        // For now, return original (this is a stub that needs full implementation)
        Ok(result)
    }

    fn format(&self) -> FileFormat {
        FileFormat::Lua
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{TranslatedEntry, FailedEntry};

    #[test]
    fn test_extract_lua() {
        let handler = LuaHandler::new();
        let content = r#"
local L = {
    greeting = "Hello World",
    farewell = 'Goodbye'
}
"#;
        
        let entries = handler.extract(content).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].source, "Hello World");
        assert_eq!(entries[1].source, "Goodbye");
    }

    #[test]
    fn test_extract_lua_with_comments() {
        let handler = LuaHandler::new();
        let content = r#"
-- Comment with "string"
local msg = "Real string"
"#;
        
        let entries = handler.extract(content).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "Real string");
    }
}

