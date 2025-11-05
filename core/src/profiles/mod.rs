/// Game profile system for game-specific translation rules
pub mod rimworld;
pub mod factorio;
pub mod stardew;
pub mod minecraft;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProfile {
    pub id: String,
    pub name: String,
    pub detector: DetectionRules,
    pub include_paths: Vec<String>,
    pub exclude_paths: Vec<String>,
    pub extra_placeholders: Vec<String>,
    pub terminology: HashMap<String, String>,
    
    /// Validator configuration (Section 9)
    #[serde(default)]
    pub validator_config: ValidatorProfileConfig,
}

/// Validator-specific profile configuration (Section 9)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorProfileConfig {
    /// Allowed token types for this game
    #[serde(default)]
    pub allowed_token_types: HashSet<String>,
    
    /// CSV column indices to translate (if applicable)
    #[serde(default)]
    pub csv_target_columns: Vec<usize>,
    
    /// Patterns that must be preserved exactly (regex)
    #[serde(default)]
    pub force_fixed_patterns: Vec<String>,
    
    /// Prohibited token substitutions
    #[serde(default)]
    pub forbidden_substitutions: Vec<TokenSubstitution>,
    
    /// Format-specific rules
    #[serde(default)]
    pub format_rules: Vec<FormatRule>,
}

/// Prohibited token substitution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenSubstitution {
    pub from: String,
    pub to: String,
    pub reason: String,
}

/// Format-specific validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatRule {
    pub format: String,
    pub rule_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionRules {
    /// Folder patterns to match
    pub folder_patterns: Vec<String>,
    
    /// File patterns to match
    pub file_patterns: Vec<String>,
    
    /// Manifest signatures
    pub manifest_signatures: Vec<String>,
}

impl GameProfile {
    /// Auto-detect game from mod path
    pub fn detect(mod_path: &std::path::Path) -> Option<GameProfile> {
        // Try each built-in profile
        if rimworld::RimWorldProfile::detect(mod_path) {
            return Some(rimworld::RimWorldProfile::profile());
        }
        
        if factorio::FactorioProfile::detect(mod_path) {
            return Some(factorio::FactorioProfile::profile());
        }
        
        if minecraft::MinecraftProfile::detect(mod_path) {
            return Some(minecraft::MinecraftProfile::profile());
        }
        
        if stardew::StardewValleyProfile::detect(mod_path) {
            return Some(stardew::StardewValleyProfile::profile());
        }
        
        None
    }
    
    /// Get generic profile (fallback)
    pub fn generic() -> Self {
        // Generic profile allows all token types
        let mut allowed_token_types = HashSet::new();
        allowed_token_types.insert("PRINTF".to_string());
        allowed_token_types.insert("DOTNET".to_string());
        allowed_token_types.insert("NAMED".to_string());
        allowed_token_types.insert("SHELL".to_string());
        allowed_token_types.insert("FACTORIO".to_string());
        allowed_token_types.insert("FLINK".to_string());
        allowed_token_types.insert("ICU".to_string());
        allowed_token_types.insert("TAG".to_string());
        allowed_token_types.insert("BBCODE".to_string());
        allowed_token_types.insert("RWCOLOR".to_string());
        allowed_token_types.insert("MCCOLOR".to_string());
        allowed_token_types.insert("RICHTEXT".to_string());
        allowed_token_types.insert("FCOLOR".to_string());
        allowed_token_types.insert("DBLBRACK".to_string());
        allowed_token_types.insert("MUSTACHE".to_string());
        allowed_token_types.insert("MATHEXPR".to_string());
        allowed_token_types.insert("RANGE".to_string());
        allowed_token_types.insert("PERCENT".to_string());
        allowed_token_types.insert("SCIENTIFIC".to_string());
        allowed_token_types.insert("UNIT".to_string());
        allowed_token_types.insert("ENTITY".to_string());
        allowed_token_types.insert("ESCAPE".to_string());
        
        let validator_config = ValidatorProfileConfig {
            allowed_token_types,
            csv_target_columns: vec![],
            force_fixed_patterns: vec![],
            forbidden_substitutions: vec![],
            format_rules: vec![],
        };
        
        Self {
            id: "generic".to_string(),
            name: "Generic Mod".to_string(),
            detector: DetectionRules {
                folder_patterns: Vec::new(),
                file_patterns: Vec::new(),
                manifest_signatures: Vec::new(),
            },
            include_paths: vec![
                "Languages/".to_string(),
                "locale/".to_string(),
                "i18n/".to_string(),
            ],
            exclude_paths: Vec::new(),
            extra_placeholders: Vec::new(),
            terminology: HashMap::new(),
            validator_config,
        }
    }
}

/// Get all available profiles
pub fn list_profiles() -> Vec<GameProfile> {
    vec![
        rimworld::RimWorldProfile::profile(),
        factorio::FactorioProfile::profile(),
        minecraft::MinecraftProfile::profile(),
        stardew::StardewValleyProfile::profile(),
        GameProfile::generic(),
    ]
}
