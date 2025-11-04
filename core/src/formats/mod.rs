/// Format handlers for different file types
/// Implements "값만 번역" (value-only translation) principle
pub mod xml;
pub mod json;
pub mod yaml;
pub mod po;
pub mod ini;
pub mod csv;
pub mod properties;
pub mod lua;
pub mod txt;

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    
    #[error("Encoding error: {0}")]
    EncodingError(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Xml,
    Json,
    Yaml,
    Po,
    Ini,
    Cfg,
    Csv,
    Properties,
    Lua,
    Txt,
    Markdown,
    Unknown,
}

impl FileFormat {
    /// Detect format from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "xml" => Self::Xml,
            "json" | "jsonl" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "po" | "pot" => Self::Po,
            "ini" => Self::Ini,
            "cfg" => Self::Cfg,
            "csv" | "tsv" => Self::Csv,
            "properties" => Self::Properties,
            "lua" => Self::Lua,
            "txt" => Self::Txt,
            "md" | "markdown" => Self::Markdown,
            _ => Self::Unknown,
        }
    }
    
    /// Detect format from path
    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(Self::from_extension)
            .unwrap_or(Self::Unknown)
    }
}

/// Represents a translatable entry (key-value pair)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatableEntry {
    /// Unique key/identifier for this entry
    pub key: String,
    
    /// Original text to translate
    pub source: String,
    
    /// Context information (file path, line number, etc.)
    pub context: Option<String>,
    
    /// Metadata (attributes, comments, etc.)
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// Result of translating entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    /// Successfully translated entries
    pub translated: Vec<TranslatedEntry>,
    
    /// Failed entries (kept as original)
    pub failed: Vec<FailedEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatedEntry {
    pub key: String,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedEntry {
    pub key: String,
    pub source: String,
    pub error: String,
}

/// Trait for format-specific handlers
pub trait FormatHandler: Send + Sync {
    /// Extract translatable entries from file content
    fn extract(&self, content: &str) -> Result<Vec<TranslatableEntry>, FormatError>;
    
    /// Merge translations back into original structure
    fn merge(
        &self,
        original: &str,
        translations: &TranslationResult,
    ) -> Result<String, FormatError>;
    
    /// Get the format this handler supports
    fn format(&self) -> FileFormat;
}

/// Get appropriate handler for a file
pub fn get_handler(format: FileFormat) -> Option<Box<dyn FormatHandler>> {
    match format {
        FileFormat::Xml => Some(Box::new(xml::XmlHandler::new())),
        FileFormat::Json => Some(Box::new(json::JsonHandler::new())),
        FileFormat::Yaml => Some(Box::new(yaml::YamlHandler::new())),
        FileFormat::Po => Some(Box::new(po::PoHandler::new())),
        FileFormat::Ini | FileFormat::Cfg => Some(Box::new(ini::IniHandler::new())),
        FileFormat::Csv => Some(Box::new(csv::CsvHandler::new())),
        FileFormat::Properties => Some(Box::new(properties::PropertiesHandler::new())),
        FileFormat::Lua => Some(Box::new(lua::LuaHandler::new())),
        FileFormat::Txt | FileFormat::Markdown => Some(Box::new(txt::TxtHandler::new())),
        FileFormat::Unknown => None,
    }
}
