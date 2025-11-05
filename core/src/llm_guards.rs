/// LLM Translation Constraints and Guards (Section 5)
/// 
/// Provides constraint strings and validation rules for LLM translation prompts
/// to ensure proper token preservation and format adherence.

use std::collections::HashSet;

/// LLM translation constraints for token preservation
#[derive(Debug, Clone)]
pub struct TranslationConstraints {
    /// Core preservation rules
    pub preserve_tokens: bool,
    /// Enforce token count and order
    pub enforce_token_order: bool,
    /// Preserve {n}% patterns
    pub preserve_percent_binding: bool,
    /// Don't translate ICU/code/LaTeX blocks
    pub protect_special_blocks: bool,
    /// Game-specific profile constraints
    pub profile_constraints: Vec<String>,
}

impl Default for TranslationConstraints {
    fn default() -> Self {
        Self {
            preserve_tokens: true,
            enforce_token_order: true,
            preserve_percent_binding: true,
            protect_special_blocks: true,
            profile_constraints: Vec::new(),
        }
    }
}

impl TranslationConstraints {
    /// Generate LLM prompt constraints based on configuration
    pub fn to_prompt(&self, protected_tokens: &[String]) -> String {
        let mut constraints = Vec::new();
        
        if self.preserve_tokens {
            constraints.push("CRITICAL: Preserve ALL protected tokens exactly as they appear.".to_string());
            
            if !protected_tokens.is_empty() {
                let tokens_list = protected_tokens.join(", ");
                constraints.push(format!(
                    "Protected tokens in this text: {}",
                    tokens_list
                ));
                constraints.push(
                    "These tokens MUST appear in your translation with EXACT same spelling and count.".to_string()
                );
            }
        }
        
        if self.enforce_token_order {
            constraints.push(
                "Maintain the relative order of protected tokens.".to_string()
            );
        }
        
        if self.preserve_percent_binding {
            constraints.push(
                "Keep format tokens bound to percent signs: {0}% must stay as {0}%, not {0} %.".to_string()
            );
            constraints.push(
                "Similarly, preserve unit bindings: 16 ms, 60 FPS must maintain the exact spacing and unit.".to_string()
            );
        }
        
        if self.protect_special_blocks {
            constraints.push(
                "Do NOT translate content inside ICU MessageFormat blocks {n, plural, ...}.".to_string()
            );
            constraints.push(
                "Do NOT translate code blocks, LaTeX formulas $...$, or technical expressions.".to_string()
            );
            constraints.push(
                "Mathematical expressions like '3.14 × r^2' and '(a+b)/2' must be preserved exactly.".to_string()
            );
        }
        
        // Add profile-specific constraints
        for constraint in &self.profile_constraints {
            constraints.push(constraint.clone());
        }
        
        // Add general translation rules
        constraints.push("".to_string());
        constraints.push("Translation rules:".to_string());
        constraints.push("- Translate ONLY natural language text between tokens.".to_string());
        constraints.push("- Do NOT modify, reorder, or remove any tokens.".to_string());
        constraints.push("- Do NOT change units, symbols, or numbers.".to_string());
        constraints.push("- Preserve all whitespace around tokens.".to_string());
        
        constraints.join("\n")
    }
    
    /// Add RimWorld-specific constraints
    pub fn with_rimworld_profile(mut self) -> Self {
        self.profile_constraints.push(
            "RimWorld: {PAWN_*} tokens have fixed spelling - never translate or modify.".to_string()
        );
        self.profile_constraints.push(
            "RimWorld: <color=#...> tags can be nested - preserve structure.".to_string()
        );
        self
    }
    
    /// Add Factorio-specific constraints
    pub fn with_factorio_profile(mut self) -> Self {
        self.profile_constraints.push(
            "Factorio: Maintain __1__, __2__, etc. in sequential order.".to_string()
        );
        self.profile_constraints.push(
            "Factorio: __ENTITY__* and __control__* names are exact - do not auto-correct.".to_string()
        );
        self.profile_constraints.push(
            "Factorio: [color=]...[/color] blocks must be balanced.".to_string()
        );
        self
    }
    
    /// Add Minecraft-specific constraints
    pub fn with_minecraft_profile(mut self) -> Self {
        self.profile_constraints.push(
            "Minecraft: Cannot convert %s to {0} or vice versa - preserve format type.".to_string()
        );
        self.profile_constraints.push(
            "Minecraft: § color codes must stay at text boundaries.".to_string()
        );
        self
    }
    
    /// Add custom constraint
    pub fn with_custom_constraint(mut self, constraint: String) -> Self {
        self.profile_constraints.push(constraint);
        self
    }
}

/// Token preservation validator for LLM output
pub struct TokenPreservationValidator {
    expected_tokens: HashSet<String>,
}

impl TokenPreservationValidator {
    pub fn new(expected_tokens: Vec<String>) -> Self {
        Self {
            expected_tokens: expected_tokens.into_iter().collect(),
        }
    }
    
    /// Check if LLM output preserved all required tokens
    pub fn validate(&self, output: &str) -> Result<(), Vec<String>> {
        let mut missing = Vec::new();
        
        for token in &self.expected_tokens {
            if !output.contains(token) {
                missing.push(token.clone());
            }
        }
        
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
    
    /// Extract tokens from text
    pub fn extract_tokens(text: &str) -> Vec<String> {
        use regex::Regex;
        use once_cell::sync::Lazy;
        
        static TOKEN_REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"⟦MT:[A-Z_]+:\d+⟧|\{[0-9]+\}|\{[A-Za-z_][A-Za-z0-9_]*\}|%[0-9]*\$?[sdifuxXoScpn]|__[A-Z]+(?:__[A-Za-z0-9_\-\.]+__)?__|__[0-9]+__|§[0-9A-FK-ORa-fk-or]")
                .expect("valid token regex")
        });
        
        TOKEN_REGEX
            .find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect()
    }
}

/// Build system prompt for translation with constraints
pub fn build_system_prompt(
    source_lang: &str,
    target_lang: &str,
    constraints: &TranslationConstraints,
    protected_tokens: &[String],
) -> String {
    let base_prompt = format!(
        "You are a professional translator specializing in game mod localization.\n\
        Your task is to translate text from {} to {}.\n\n",
        source_lang, target_lang
    );
    
    let constraints_text = constraints.to_prompt(protected_tokens);
    
    format!(
        "{}\n\
        IMPORTANT CONSTRAINTS:\n\
        {}\n\n\
        Provide ONLY the translated text, without any explanations or notes.",
        base_prompt, constraints_text
    )
}

/// Build user prompt for a specific translation segment
pub fn build_user_prompt(text: &str, context: Option<&str>) -> String {
    if let Some(ctx) = context {
        format!(
            "Context: {}\n\n\
            Text to translate:\n\
            {}",
            ctx, text
        )
    } else {
        format!("Translate:\n{}", text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_constraints() {
        let constraints = TranslationConstraints::default();
        let tokens = vec!["⟦MT:TAG:0⟧".to_string(), "{0}".to_string()];
        let prompt = constraints.to_prompt(&tokens);
        
        assert!(prompt.contains("CRITICAL"));
        assert!(prompt.contains("⟦MT:TAG:0⟧"));
        assert!(prompt.contains("{0}"));
        assert!(prompt.contains("preserve"));
    }
    
    #[test]
    fn test_rimworld_profile() {
        let constraints = TranslationConstraints::default().with_rimworld_profile();
        let tokens = vec!["{PAWN_label}".to_string()];
        let prompt = constraints.to_prompt(&tokens);
        
        assert!(prompt.contains("RimWorld"));
        assert!(prompt.contains("{PAWN_"));
    }
    
    #[test]
    fn test_factorio_profile() {
        let constraints = TranslationConstraints::default().with_factorio_profile();
        let tokens = vec!["__1__".to_string(), "__ENTITY__iron-ore__".to_string()];
        let prompt = constraints.to_prompt(&tokens);
        
        assert!(prompt.contains("Factorio"));
        assert!(prompt.contains("__1__"));
        assert!(prompt.contains("sequential order"));
    }
    
    #[test]
    fn test_token_extraction() {
        let text = "Hello ⟦MT:TAG:0⟧ {0} world %s";
        let tokens = TokenPreservationValidator::extract_tokens(text);
        
        assert!(tokens.len() >= 3);
        assert!(tokens.contains(&"⟦MT:TAG:0⟧".to_string()));
        assert!(tokens.contains(&"{0}".to_string()));
        assert!(tokens.contains(&"%s".to_string()));
    }
    
    #[test]
    fn test_token_validation() {
        let expected = vec!["⟦MT:TAG:0⟧".to_string(), "{0}".to_string()];
        let validator = TokenPreservationValidator::new(expected);
        
        // Valid case
        assert!(validator.validate("Text ⟦MT:TAG:0⟧ with {0} tokens").is_ok());
        
        // Missing token case
        let result = validator.validate("Text ⟦MT:TAG:0⟧ without second token");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), vec!["{0}"]);
    }
    
    #[test]
    fn test_build_system_prompt() {
        let constraints = TranslationConstraints::default();
        let tokens = vec!["⟦MT:TAG:0⟧".to_string()];
        let prompt = build_system_prompt("English", "Korean", &constraints, &tokens);
        
        assert!(prompt.contains("English"));
        assert!(prompt.contains("Korean"));
        assert!(prompt.contains("IMPORTANT CONSTRAINTS"));
        assert!(prompt.contains("⟦MT:TAG:0⟧"));
    }
    
    #[test]
    fn test_build_user_prompt() {
        let text = "Hello world";
        let prompt = build_user_prompt(text, None);
        assert!(prompt.contains("Hello world"));
        
        let prompt_with_ctx = build_user_prompt(text, Some("greeting"));
        assert!(prompt_with_ctx.contains("Context: greeting"));
        assert!(prompt_with_ctx.contains("Hello world"));
    }
    
    #[test]
    fn test_percent_binding_constraint() {
        let constraints = TranslationConstraints {
            preserve_percent_binding: true,
            ..Default::default()
        };
        let prompt = constraints.to_prompt(&[]);
        
        assert!(prompt.contains("{0}%"));
        assert!(prompt.contains("unit"));
    }
    
    #[test]
    fn test_special_blocks_protection() {
        let constraints = TranslationConstraints {
            protect_special_blocks: true,
            ..Default::default()
        };
        let prompt = constraints.to_prompt(&[]);
        
        assert!(prompt.contains("ICU MessageFormat"));
        assert!(prompt.contains("LaTeX"));
        assert!(prompt.contains("Mathematical expressions"));
    }
}
