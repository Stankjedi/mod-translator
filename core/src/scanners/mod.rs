/// Format-specific scanners for token protection
/// 
/// Each scanner implements token protection for a specific file format,
/// identifying and protecting non-translatable elements while preserving
/// translatable natural language text.

pub mod markdown;
pub mod properties;
pub mod lua;

pub use markdown::MarkdownScanner;
pub use properties::PropertiesScanner;
pub use lua::LuaScanner;
