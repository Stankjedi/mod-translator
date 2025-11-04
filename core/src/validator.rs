/// Enhanced validation for translation quality
use crate::protector::{ProtectedFragment, ProtectorError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationError {
    /// XML/JSON structure changed
    StructureMismatch,
    
    /// Tag set doesn't match
    TagSetMismatch,
    
    /// Placeholder count/order mismatch
    PlaceholderMismatch,
    
    /// Pipe delimiter count mismatch
    PipeDelimMismatch,
    
    /// Entity/escape sequence changed
    EscapeEntityDrift,
    
    /// Result is empty
    EmptyValue,
    
    /// Length is suspiciously long (4x+ original)
    OverlongDelta,
    
    /// Illegal backtick in output
    IllegalBacktick,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn pass() -> Self {
        Self {
            passed: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    pub fn fail(error: ValidationError) -> Self {
        Self {
            passed: false,
            errors: vec![error],
            warnings: Vec::new(),
        }
    }
    
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }
}

pub struct Validator;

impl Validator {
    /// Validate that protected tokens match between source and target
    pub fn validate_tokens(
        source_fragment: &ProtectedFragment,
        translated: &str,
    ) -> ValidationResult {
        match source_fragment.restore(translated) {
            Ok(_) => ValidationResult::pass(),
            Err(ProtectorError::MissingTokens(tokens)) => {
                let mut result = ValidationResult::fail(ValidationError::PlaceholderMismatch);
                result.warnings.push(format!("Missing tokens: {}", tokens.join(", ")));
                result
            }
            Err(ProtectorError::UnexpectedTokens(tokens)) => {
                let mut result = ValidationResult::fail(ValidationError::PlaceholderMismatch);
                result.warnings.push(format!("Unexpected tokens: {}", tokens.join(", ")));
                result
            }
        }
    }
    
    /// Validate empty result
    pub fn validate_not_empty(translated: &str) -> ValidationResult {
        if translated.trim().is_empty() {
            ValidationResult::fail(ValidationError::EmptyValue)
        } else {
            ValidationResult::pass()
        }
    }
    
    /// Validate length (warn if >4x original)
    pub fn validate_length(source: &str, translated: &str) -> ValidationResult {
        let source_len = source.len();
        let translated_len = translated.len();
        
        if source_len > 0 && translated_len > source_len * 4 {
            ValidationResult::pass().with_warning(format!(
                "Translation is {}x longer than source ({} -> {})",
                translated_len / source_len,
                source_len,
                translated_len
            ))
        } else {
            ValidationResult::pass()
        }
    }
    
    /// Validate pipe delimiter count matches
    pub fn validate_pipe_count(source: &str, translated: &str) -> ValidationResult {
        let source_pipes = source.matches('|').count();
        let translated_pipes = translated.matches('|').count();
        
        if source_pipes != translated_pipes {
            let mut result = ValidationResult::fail(ValidationError::PipeDelimMismatch);
            result.warnings.push(format!(
                "Pipe count mismatch: source has {}, translation has {}",
                source_pipes, translated_pipes
            ));
            result
        } else {
            ValidationResult::pass()
        }
    }
    
    /// Validate no illegal backticks in code
    pub fn validate_no_backticks(translated: &str) -> ValidationResult {
        if translated.contains('`') {
            ValidationResult::fail(ValidationError::IllegalBacktick)
        } else {
            ValidationResult::pass()
        }
    }
    
    /// Run all validations
    pub fn validate_all(
        source: &str,
        source_fragment: &ProtectedFragment,
        translated: &str,
    ) -> ValidationResult {
        let mut result = ValidationResult::pass();
        
        // Check empty
        let empty_check = Self::validate_not_empty(translated);
        if !empty_check.passed {
            return empty_check;
        }
        
        // Check tokens
        let token_check = Self::validate_tokens(source_fragment, translated);
        if !token_check.passed {
            result.passed = false;
            result.errors.extend(token_check.errors);
            result.warnings.extend(token_check.warnings);
        }
        
        // Check pipe count
        let pipe_check = Self::validate_pipe_count(source, translated);
        if !pipe_check.passed {
            result.passed = false;
            result.errors.extend(pipe_check.errors);
            result.warnings.extend(pipe_check.warnings);
        }
        
        // Check length (warning only)
        let length_check = Self::validate_length(source, translated);
        result.warnings.extend(length_check.warnings);
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protector::Protector;
    
    #[test]
    fn validates_empty_translation() {
        let result = Validator::validate_not_empty("");
        assert!(!result.passed);
        assert_eq!(result.errors[0], ValidationError::EmptyValue);
    }
    
    #[test]
    fn validates_pipe_count() {
        let result = Validator::validate_pipe_count("a|b|c", "x|y");
        assert!(!result.passed);
        assert_eq!(result.errors[0], ValidationError::PipeDelimMismatch);
    }
    
    #[test]
    fn validates_token_preservation() {
        let fragment = Protector::protect("Hello {0} world");
        let result = Validator::validate_tokens(&fragment, "Bonjour monde");
        assert!(!result.passed);
        assert_eq!(result.errors[0], ValidationError::PlaceholderMismatch);
    }
    
    #[test]
    fn passes_valid_translation() {
        let source = "Hello {0} world";
        let fragment = Protector::protect(source);
        let masked = fragment.masked_text();
        let translated = masked.replace("Hello", "Bonjour").replace("world", "monde");
        
        let result = Validator::validate_all(source, &fragment, &translated);
        assert!(result.passed);
    }
}
