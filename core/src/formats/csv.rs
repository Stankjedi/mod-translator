/// CSV format handler (stub implementation)
use super::{FileFormat, FormatError, FormatHandler, TranslatableEntry, TranslationResult};

pub struct CsvHandler;

impl CsvHandler {
    pub fn new() -> Self {
        Self
    }
}

impl FormatHandler for CsvHandler {
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
        FileFormat::Csv
    }
}
