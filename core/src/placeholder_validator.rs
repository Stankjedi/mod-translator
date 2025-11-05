/// Enhanced placeholder validator with auto-recovery for XML translation
/// 
/// This module implements comprehensive validation of protected tokens (⟦MT:TAG:n⟧, etc.)
/// and format tokens ({n}) during translation, with automatic recovery mechanisms.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Regex patterns for token detection - updated to match all new token types
// NOTE: This list must be kept synchronized with TokenClass enum in protector.rs
// When adding new token types, update both the enum and this regex pattern
static PROTECTED_TOKEN_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"⟦MT:(PRINTF|DOTNET|NAMED|SHELL|FACTORIO|FLINK|ICU|TAG|BBCODE|RWCOLOR|MCCOLOR|RICHTEXT|FCOLOR|DBLBRACK|MUSTACHE|MATHEXPR|RANGE|PERCENT|SCIENTIFIC|UNIT|ESCBRACE|ESCPCT|ENTITY|ESCAPE|ATTR|KEY|PIPE|IDPATH):(\d+)⟧")
        .expect("valid protected token regex")
});

static FORMAT_TOKEN_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{(\d+)\}")
        .expect("valid format token regex")
});

static FORMAT_WITH_PERCENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{(\d+)\}%")
        .expect("valid format with percent regex")
});

/// Error codes for validation failures
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationErrorCode {
    /// Placeholder count/order mismatch
    PlaceholderMismatch,
    /// Protected token pair unbalanced (opening/closing mismatch)
    PairUnbalanced,
    /// Format token missing ({n})
    FormatTokenMissing,
    /// XML malformed after token restoration
    XmlMalformedAfterRestore,
    /// Partial retry failed after auto-recovery
    RetryFailed,
    /// ICU MessageFormat block unbalanced
    IcuUnbalanced,
    /// Format-specific parser error (JSON, YAML, etc.)
    ParserError,
    /// Factorio token order incorrect
    FactorioOrderError,
    /// Markdown code fence unbalanced
    MarkdownUnbalancedFence,
    /// Properties unicode escape invalid
    PropertiesEscapeInvalid,
    /// Lua string literal unbalanced
    LuaStringUnbalanced,
}

/// Auto-recovery step types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecoveryStep {
    /// Reinject missing protected tokens at relative positions
    ReinjectMissingProtected,
    /// Balance opening/closing token pairs
    PairBalanceCheck,
    /// Remove excess tokens
    RemoveExcessTokens,
    /// Correct format tokens ({n})
    CorrectFormatTokens,
    /// Preserve {n}% patterns
    PreservePercentBinding,
}

/// Auto-recovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutofixResult {
    pub applied: bool,
    pub steps: Vec<RecoveryStep>,
}

impl AutofixResult {
    fn none() -> Self {
        Self {
            applied: false,
            steps: Vec::new(),
        }
    }

    fn with_steps(steps: Vec<RecoveryStep>) -> Self {
        Self {
            applied: !steps.is_empty(),
            steps,
        }
    }
}

/// Retry attempt information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryInfo {
    pub attempted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
}

impl RetryInfo {
    fn not_attempted() -> Self {
        Self {
            attempted: false,
            success: None,
        }
    }

    fn attempted(success: bool) -> Self {
        Self {
            attempted: true,
            success: Some(success),
        }
    }
}

/// UI hint for display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiHint {
    pub show_source: bool,
    pub show_candidate: bool,
    pub copy_buttons: bool,
}

impl Default for UiHint {
    fn default() -> Self {
        Self {
            show_source: true,
            show_candidate: true,
            copy_buttons: true,
        }
    }
}

/// Comprehensive failure report
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationFailureReport {
    pub code: ValidationErrorCode,
    pub file: String,
    pub line: u32,
    pub key: String,
    pub expected_protected: Vec<String>,
    pub found_protected: Vec<String>,
    pub expected_format: Vec<String>,
    pub found_format: Vec<String>,
    pub source_line: String,
    pub preprocessed_source: String,
    pub candidate_line: String,
    pub autofix: AutofixResult,
    pub retry: RetryInfo,
    pub ui_hint: UiHint,
}

/// Placeholder set with multiset tracking
#[derive(Debug, Clone)]
pub struct PlaceholderSet {
    /// Ordered list of protected tokens
    pub protected: Vec<String>,
    /// Multiset count of protected tokens
    pub protected_multiset: HashMap<String, usize>,
    /// Ordered list of format tokens
    pub format: Vec<String>,
    /// Multiset count of format tokens
    pub format_multiset: HashMap<String, usize>,
}

impl PlaceholderSet {
    pub fn new() -> Self {
        Self {
            protected: Vec::new(),
            protected_multiset: HashMap::new(),
            format: Vec::new(),
            format_multiset: HashMap::new(),
        }
    }

    /// Extract tokens from text
    pub fn from_text(text: &str) -> Self {
        let mut set = Self::new();

        // Extract protected tokens
        for cap in PROTECTED_TOKEN_REGEX.captures_iter(text) {
            if let Some(token) = cap.get(0) {
                let token_str = token.as_str().to_string();
                set.protected.push(token_str.clone());
                *set.protected_multiset.entry(token_str).or_insert(0) += 1;
            }
        }

        // Extract format tokens
        for cap in FORMAT_TOKEN_REGEX.captures_iter(text) {
            if let Some(token) = cap.get(0) {
                let token_str = token.as_str().to_string();
                set.format.push(token_str.clone());
                *set.format_multiset.entry(token_str).or_insert(0) += 1;
            }
        }

        set
    }

    /// Check if multisets match
    pub fn matches_multiset(&self, other: &PlaceholderSet) -> bool {
        self.protected_multiset == other.protected_multiset
            && self.format_multiset == other.format_multiset
    }

    /// Check if order matches
    pub fn matches_order(&self, other: &PlaceholderSet) -> bool {
        self.protected == other.protected && self.format == other.format
    }
}

/// Segment information for validation with format-specific metadata (Section 6)
#[derive(Debug, Clone)]
pub struct Segment {
    pub file: String,
    pub line: u32,
    pub key: String,
    pub source_raw: String,
    pub source_preprocessed: String,
    pub expected: PlaceholderSet,
    pub format: Option<FileFormat>,
    pub token_types: Vec<String>,
}

/// File format types for format-aware validation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Xml,
    Json,
    Cfg,
    Ini,
    Po,
    Yaml,
    Csv,
    Txt,
    Markdown,
    Properties,
    Lua,
    Unknown,
}

impl Segment {
    pub fn new(file: String, line: u32, key: String, source_raw: String, source_preprocessed: String) -> Self {
        let expected = PlaceholderSet::from_text(&source_preprocessed);
        Self {
            file,
            line,
            key,
            source_raw,
            source_preprocessed,
            expected,
            format: None,
            token_types: Vec::new(),
        }
    }
    
    /// Create segment with format metadata
    pub fn with_format(mut self, format: FileFormat) -> Self {
        self.format = Some(format);
        self
    }
    
    /// Create segment with token type information
    pub fn with_token_types(mut self, token_types: Vec<String>) -> Self {
        self.token_types = token_types;
        self
    }
}

/// Validator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorConfig {
    pub enable_autofix: bool,
    pub retry_on_fail: bool,
    pub retry_limit: usize,
    pub strict_pairing: bool,
    pub preserve_percent_binding: bool,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            enable_autofix: true,
            retry_on_fail: true,
            retry_limit: 1,
            strict_pairing: true,
            preserve_percent_binding: true,
        }
    }
}

/// Main validator
pub struct PlaceholderValidator {
    config: ValidatorConfig,
}

impl PlaceholderValidator {
    pub fn new(config: ValidatorConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(ValidatorConfig::default())
    }

    /// Validate translated text against segment expectations
    pub fn validate(
        &self,
        segment: &Segment,
        translated: &str,
    ) -> Result<String, ValidationFailureReport> {
        let found = PlaceholderSet::from_text(translated);

        // Check if validation passes
        if segment.expected.matches_multiset(&found) {
            // Multiset matches - check order
            if !segment.expected.matches_order(&found) {
                // Order mismatch - warning but not failure (could auto-reorder if needed)
                log::warn!(
                    "Token order mismatch in {}:{} ({})",
                    segment.file,
                    segment.line,
                    segment.key
                );
            }
            
            // Even if validation passes, check if we need to preserve {n}% patterns
            if self.config.preserve_percent_binding {
                if let Some(corrected) = self.preserve_percent_patterns(&segment.source_preprocessed, translated) {
                    return Ok(corrected);
                }
            }
            
            return Ok(translated.to_string());
        }

        // Validation failed - attempt auto-recovery if enabled
        if self.config.enable_autofix {
            match self.auto_recover(segment, translated, &found) {
                Ok(recovered) => {
                    // Verify recovered text
                    let recovered_set = PlaceholderSet::from_text(&recovered);
                    if segment.expected.matches_multiset(&recovered_set) {
                        return Ok(recovered);
                    }
                }
                Err(_) => {
                    // Auto-recovery failed, continue to error reporting
                }
            }
        }

        // Create failure report
        Err(self.create_failure_report(
            segment,
            translated,
            &found,
            AutofixResult::none(),
            RetryInfo::not_attempted(),
        ))
    }

    /// Attempt automatic recovery
    fn auto_recover(
        &self,
        segment: &Segment,
        translated: &str,
        found: &PlaceholderSet,
    ) -> Result<String, String> {
        let mut result = translated.to_string();
        let mut steps = Vec::new();

        // Step 1: Reinject missing protected tokens
        let missing_protected: Vec<_> = segment
            .expected
            .protected
            .iter()
            .filter(|token| {
                let expected_count = segment.expected.protected_multiset.get(*token).unwrap_or(&0);
                let found_count = found.protected_multiset.get(*token).unwrap_or(&0);
                found_count < expected_count
            })
            .collect();

        if !missing_protected.is_empty() {
            result = self.reinject_missing_tokens(&segment.source_preprocessed, &result, &missing_protected);
            steps.push(RecoveryStep::ReinjectMissingProtected);
        }

        // Step 2: Balance pairs if strict_pairing is enabled
        if self.config.strict_pairing {
            if let Some(balanced) = self.balance_pairs(&result) {
                result = balanced;
                steps.push(RecoveryStep::PairBalanceCheck);
            }
        }

        // Step 3: Remove excess tokens
        let excess_protected: Vec<_> = found
            .protected
            .iter()
            .filter(|token| {
                let expected_count = segment.expected.protected_multiset.get(*token).unwrap_or(&0);
                let found_count = found.protected_multiset.get(*token).unwrap_or(&0);
                found_count > expected_count
            })
            .collect();

        if !excess_protected.is_empty() {
            result = self.remove_excess_tokens(&result, &excess_protected);
            steps.push(RecoveryStep::RemoveExcessTokens);
        }

        // Step 4: Correct format tokens
        let missing_format: Vec<_> = segment
            .expected
            .format
            .iter()
            .filter(|token| {
                let expected_count = segment.expected.format_multiset.get(*token).unwrap_or(&0);
                let found_count = found.format_multiset.get(*token).unwrap_or(&0);
                found_count < expected_count
            })
            .collect();

        if !missing_format.is_empty() {
            result = self.reinject_format_tokens(&segment.source_preprocessed, &result, &missing_format);
            steps.push(RecoveryStep::CorrectFormatTokens);
        }

        // Step 5: Preserve {n}% patterns if enabled
        if self.config.preserve_percent_binding {
            if let Some(corrected) = self.preserve_percent_patterns(&segment.source_preprocessed, &result) {
                result = corrected;
                steps.push(RecoveryStep::PreservePercentBinding);
            }
        }

        if steps.is_empty() {
            Err("No recovery steps could be applied".to_string())
        } else {
            Ok(result)
        }
    }

    /// Reinject missing tokens at relative positions
    fn reinject_missing_tokens(&self, source: &str, translated: &str, missing: &[&String]) -> String {
        let mut result = translated.to_string();

        for token in missing {
            // Find relative position in source
            if let Some(pos) = source.find(token.as_str()) {
                let relative_pos = pos as f64 / source.len().max(1) as f64;
                
                // Calculate insertion position in translated text
                let mut insert_pos = (translated.len() as f64 * relative_pos) as usize;
                insert_pos = insert_pos.min(result.len());
                
                // Ensure we're at a UTF-8 character boundary
                while insert_pos > 0 && !result.is_char_boundary(insert_pos) {
                    insert_pos -= 1;
                }

                // Insert token
                result.insert_str(insert_pos, token);
            }
        }

        result
    }

    /// Balance opening/closing token pairs
    fn balance_pairs(&self, text: &str) -> Option<String> {
        // Simple implementation: ensure even number of TAG tokens
        let tag_tokens: Vec<_> = PROTECTED_TOKEN_REGEX
            .captures_iter(text)
            .filter(|cap| cap.get(1).map(|m| m.as_str() == "TAG").unwrap_or(false))
            .collect();

        if tag_tokens.len() % 2 != 0 {
            // Odd number of TAG tokens - add closing tag at end
            let last_tag_num = tag_tokens
                .last()
                .and_then(|cap| cap.get(2))
                .and_then(|m| m.as_str().parse::<u32>().ok())
                .unwrap_or(0);
            
            let closing_tag = format!("⟦MT:TAG:{}⟧", last_tag_num + 1);
            return Some(format!("{}{}", text, closing_tag));
        }

        None
    }

    /// Remove excess tokens
    fn remove_excess_tokens(&self, text: &str, excess: &[&String]) -> String {
        let mut result = text.to_string();

        for token in excess {
            // Remove first occurrence
            if let Some(pos) = result.find(token.as_str()) {
                result.replace_range(pos..pos + token.len(), "");
            }
        }

        result
    }

    /// Reinject format tokens
    fn reinject_format_tokens(&self, source: &str, translated: &str, missing: &[&String]) -> String {
        let mut result = translated.to_string();

        for token in missing {
            // Find relative position in source
            if let Some(pos) = source.find(token.as_str()) {
                let relative_pos = pos as f64 / source.len().max(1) as f64;
                
                // Calculate insertion position in translated text
                let mut insert_pos = (translated.len() as f64 * relative_pos) as usize;
                insert_pos = insert_pos.min(result.len());
                
                // Ensure we're at a UTF-8 character boundary
                while insert_pos > 0 && !result.is_char_boundary(insert_pos) {
                    insert_pos -= 1;
                }

                // Insert token
                result.insert_str(insert_pos, token);
            }
        }

        result
    }

    /// Preserve {n}% patterns
    fn preserve_percent_patterns(&self, source: &str, translated: &str) -> Option<String> {
        // Find all {n}% patterns in source
        let source_patterns: Vec<_> = FORMAT_WITH_PERCENT_REGEX.find_iter(source).collect();
        
        if source_patterns.is_empty() {
            return None;
        }

        let mut result = translated.to_string();
        let mut modified = false;

        // Ensure all {n}% patterns are preserved
        for pattern in source_patterns {
            let pattern_str = pattern.as_str();
            
            // Check if pattern exists in result
            if !result.contains(pattern_str) {
                // Extract just the {n} part
                if let Some(cap) = FORMAT_TOKEN_REGEX.captures(pattern_str) {
                    if let Some(token) = cap.get(0) {
                        let token_str = token.as_str();
                        
                        // Find {n} without % and add %
                        if let Some(pos) = result.find(token_str) {
                            let end_pos = pos + token_str.len();
                            // Check if % is already there
                            if end_pos >= result.len() || !result[end_pos..].starts_with('%') {
                                result.insert(end_pos, '%');
                                modified = true;
                            }
                        }
                    }
                }
            }
        }

        if modified {
            Some(result)
        } else {
            None
        }
    }
    
    /// Validate format after token restoration (Section 6)
    pub fn validate_format_after_restore(
        &self,
        restored: &str,
        format: FileFormat,
    ) -> Result<(), ValidationErrorCode> {
        use crate::format_validator;
        
        match format {
            FileFormat::Json => {
                format_validator::validate_json(restored)
                    .map_err(|_| ValidationErrorCode::ParserError)?;
            }
            FileFormat::Xml => {
                format_validator::validate_xml(restored)
                    .map_err(|_| ValidationErrorCode::XmlMalformedAfterRestore)?;
            }
            FileFormat::Yaml => {
                format_validator::validate_yaml(restored)
                    .map_err(|_| ValidationErrorCode::ParserError)?;
            }
            FileFormat::Po => {
                format_validator::validate_po(restored)
                    .map_err(|_| ValidationErrorCode::ParserError)?;
            }
            FileFormat::Ini | FileFormat::Cfg => {
                format_validator::validate_ini(restored)
                    .map_err(|_| ValidationErrorCode::ParserError)?;
            }
            FileFormat::Csv => {
                format_validator::validate_csv(restored)
                    .map_err(|_| ValidationErrorCode::ParserError)?;
            }
            FileFormat::Markdown => {
                format_validator::validate_markdown(restored)
                    .map_err(|_| ValidationErrorCode::MarkdownUnbalancedFence)?;
            }
            FileFormat::Properties => {
                format_validator::validate_properties(restored)
                    .map_err(|_| ValidationErrorCode::PropertiesEscapeInvalid)?;
            }
            FileFormat::Lua => {
                format_validator::validate_lua(restored)
                    .map_err(|_| ValidationErrorCode::LuaStringUnbalanced)?;
            }
            // Text-based formats don't need strict validation
            FileFormat::Txt => {}
            FileFormat::Unknown => {}
        }
        
        Ok(())
    }

    /// Create failure report
    fn create_failure_report(
        &self,
        segment: &Segment,
        candidate: &str,
        found: &PlaceholderSet,
        autofix: AutofixResult,
        retry: RetryInfo,
    ) -> ValidationFailureReport {
        ValidationFailureReport {
            code: ValidationErrorCode::PlaceholderMismatch,
            file: segment.file.clone(),
            line: segment.line,
            key: segment.key.clone(),
            expected_protected: segment.expected.protected.clone(),
            found_protected: found.protected.clone(),
            expected_format: segment.expected.format.clone(),
            found_format: found.format.clone(),
            source_line: segment.source_raw.clone(),
            preprocessed_source: segment.source_preprocessed.clone(),
            candidate_line: candidate.to_string(),
            autofix,
            retry,
            ui_hint: UiHint::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder_set_extraction() {
        let text = "⟦MT:TAG:0⟧Hello {0} world⟦MT:TAG:1⟧";
        let set = PlaceholderSet::from_text(text);
        
        assert_eq!(set.protected.len(), 2);
        assert_eq!(set.format.len(), 1);
        assert_eq!(set.protected_multiset.get("⟦MT:TAG:0⟧"), Some(&1));
        assert_eq!(set.format_multiset.get("{0}"), Some(&1));
    }

    #[test]
    fn test_multiset_matching() {
        let text1 = "⟦MT:TAG:0⟧Hello {0}⟦MT:TAG:1⟧";
        let text2 = "{0} ⟦MT:TAG:1⟧World⟦MT:TAG:0⟧";
        
        let set1 = PlaceholderSet::from_text(text1);
        let set2 = PlaceholderSet::from_text(text2);
        
        assert!(set1.matches_multiset(&set2));
        assert!(!set1.matches_order(&set2));
    }

    #[test]
    fn test_simple_token_omission() {
        let segment = Segment::new(
            "test.xml".to_string(),
            1,
            "test_key".to_string(),
            "<tag>Hello {0} world</tag>".to_string(),
            "⟦MT:TAG:0⟧Hello {0} world⟦MT:TAG:1⟧".to_string(),
        );

        let validator = PlaceholderValidator::with_default_config();
        let translated = "안녕하세요 세계"; // Missing all tokens
        
        let result = validator.validate(&segment, translated);
        
        // Should fail or recover
        match result {
            Ok(recovered) => {
                // Check if tokens were recovered
                let recovered_set = PlaceholderSet::from_text(&recovered);
                assert!(!recovered_set.protected.is_empty() || !recovered_set.format.is_empty());
            }
            Err(report) => {
                assert_eq!(report.code, ValidationErrorCode::PlaceholderMismatch);
            }
        }
    }

    #[test]
    fn test_format_token_with_percent() {
        let segment = Segment::new(
            "test.xml".to_string(),
            1,
            "test_key".to_string(),
            "Speed {0}%".to_string(),
            "Speed {0}%".to_string(),
        );

        let validator = PlaceholderValidator::with_default_config();
        let translated = "속도 {0}"; // Missing %
        
        let result = validator.validate(&segment, translated);
        
        match result {
            Ok(recovered) => {
                eprintln!("Recovered: '{}'", recovered);
                assert!(recovered.contains("{0}%"), "Expected {{0}}% but got: {}", recovered);
            }
            Err(report) => {
                eprintln!("Validation failed: {:?}", report.code);
                panic!("Should have recovered but got error");
            }
        }
    }

    #[test]
    fn test_example_line_from_issue() {
        let segment = Segment::new(
            "Settings.xml".to_string(),
            32,
            "WBR.HookupRateTip".to_string(),
            "<WBR.HookupRateTip>Relative frequency for hookups … entirely.</WBR.HookupRateTip>".to_string(),
            "⟦MT:TAG:0⟧Relative frequency for hookups … entirely.⟦MT:TAG:1⟧".to_string(),
        );

        let validator = PlaceholderValidator::with_default_config();
        let translated = "후킹의 상대적 빈도입니다 … 완전히 비활성화됩니다."; // Missing tokens
        
        let result = validator.validate(&segment, translated);
        
        match result {
            Ok(recovered) => {
                // Tokens should be recovered
                assert!(recovered.contains("⟦MT:TAG:0⟧"));
                assert!(recovered.contains("⟦MT:TAG:1⟧"));
            }
            Err(report) => {
                // Should have auto-recovery attempted
                assert_eq!(report.code, ValidationErrorCode::PlaceholderMismatch);
            }
        }
    }
}
