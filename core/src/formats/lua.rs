/// Lua format handler (stub implementation)
use super::{FileFormat, FormatError, FormatHandler, TranslatableEntry, TranslationResult};

pub struct LuaHandler;

impl LuaHandler {
    pub fn new() -> Self {
        Self
    }
}

impl FormatHandler for LuaHandler {
    fn extract(&self, _content: &str) -> Result<Vec<TranslatableEntry>, FormatError> {
        Ok(Vec::new())
    }

    fn merge(
        &self,
        original: &str,
        _translations: &TranslationResult,
    ) -> Result<String, FormatError> {
        Ok(original.to_string())
    }

    fn format(&self) -> FileFormat {
        FileFormat::Lua
    }
}
