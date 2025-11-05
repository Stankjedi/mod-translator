/// Factorio game profile
use super::{DetectionRules, GameProfile, ValidatorProfileConfig, FormatRule, TokenSubstitution};
use std::path::Path;
use std::collections::{HashMap, HashSet};

pub struct FactorioProfile;

impl FactorioProfile {
    pub fn detect(mod_path: &Path) -> bool {
        // Check for info.json (Factorio mod manifest)
        let info_json = mod_path.join("info.json");
        if info_json.exists() {
            return true;
        }
        
        // Check for locale/ directory
        let locale = mod_path.join("locale");
        if locale.exists() && locale.is_dir() {
            return true;
        }
        
        false
    }
    
    pub fn profile() -> GameProfile {
        // Validator configuration for Factorio (Section 3)
        let mut allowed_token_types = HashSet::new();
        allowed_token_types.insert("FACTORIO".to_string());    // __1__, __ENTITY__*
        allowed_token_types.insert("FLINK".to_string());       // [img=item/plate]
        allowed_token_types.insert("FCOLOR".to_string());      // [color=red]
        allowed_token_types.insert("BBCODE".to_string());      // [b], [i]
        allowed_token_types.insert("PRINTF".to_string());      // %s
        allowed_token_types.insert("PERCENT".to_string());     // percentages
        allowed_token_types.insert("UNIT".to_string());        // units with numbers
        allowed_token_types.insert("MATHEXPR".to_string());    // math expressions
        allowed_token_types.insert("RANGE".to_string());       // ranges
        allowed_token_types.insert("SCIENTIFIC".to_string());  // scientific notation
        
        let validator_config = ValidatorProfileConfig {
            allowed_token_types,
            csv_target_columns: vec![],
            force_fixed_patterns: vec![
                // Numeric macros: __1__, __2__, etc.
                r"__[0-9]+__".to_string(),
                // Entity macros: __ENTITY__iron-ore__
                r"__[A-Z]+__[A-Za-z0-9_\-\.]+__".to_string(),
                // Control macros: __control__inventory__
                r"__control__[A-Za-z0-9_\-\.]+__".to_string(),
            ],
            forbidden_substitutions: vec![
                TokenSubstitution {
                    from: "{0}".to_string(),
                    to: "%s".to_string(),
                    reason: "Factorio uses printf format, but don't convert existing .NET format".to_string(),
                },
            ],
            format_rules: vec![
                FormatRule {
                    format: "cfg".to_string(),
                    rule_type: "factorio_macro_order".to_string(),
                    description: "Preserve order of __1__, __2__, etc.".to_string(),
                },
                FormatRule {
                    format: "cfg".to_string(),
                    rule_type: "control_names_exact".to_string(),
                    description: "No auto-correction for __control__* names".to_string(),
                },
                FormatRule {
                    format: "cfg".to_string(),
                    rule_type: "color_block_preservation".to_string(),
                    description: "Preserve [color=]...[/color] block structure".to_string(),
                },
                FormatRule {
                    format: "cfg".to_string(),
                    rule_type: "image_link_preservation".to_string(),
                    description: "[img=...] must preserve full path".to_string(),
                },
            ],
        };
        
        GameProfile {
            id: "factorio".to_string(),
            name: "Factorio".to_string(),
            detector: DetectionRules {
                folder_patterns: vec!["locale/".to_string()],
                file_patterns: vec!["info.json".to_string()],
                manifest_signatures: vec!["\"factorio_version\"".to_string()],
            },
            include_paths: vec!["locale/".to_string()],
            exclude_paths: vec![
                "graphics/".to_string(),
                "sounds/".to_string(),
            ],
            extra_placeholders: vec![
                r"__[A-Z_]+__".to_string(),
                r"%\d*s".to_string(),
            ],
            terminology: HashMap::new(),
            validator_config,
        }
    }
}
