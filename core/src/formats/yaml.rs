/// YAML format handler (stub implementation)
use super::{FileFormat, FormatError, FormatHandler, TranslatableEntry, TranslationResult};

pub struct YamlHandler;

impl YamlHandler {
    pub fn new() -> Self {
        Self
    }
}

impl FormatHandler for YamlHandler {
    fn extract(&self, _content: &str) -> Result<Vec<TranslatableEntry>, FormatError> {
        // Placeholder implementation
        Ok(Vec::new())
    }

    fn merge(
        &self,
        original: &str,
        _translations: &TranslationResult,
    ) -> Result<String, FormatError> {
        // Placeholder: return original
        Ok(original.to_string())
    }

    fn format(&self) -> FileFormat {
        FileFormat::Yaml
    }
}
