/// Factorio game profile
use super::{DetectionRules, GameProfile, ValidatorProfileConfig, FormatRule};
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
        // Validator configuration for Factorio
        let mut allowed_token_types = HashSet::new();
        allowed_token_types.insert("FACTORIO".to_string());
        allowed_token_types.insert("FLINK".to_string());
        allowed_token_types.insert("FCOLOR".to_string());
        allowed_token_types.insert("BBCODE".to_string());
        allowed_token_types.insert("PRINTF".to_string());
        
        let validator_config = ValidatorProfileConfig {
            allowed_token_types,
            csv_target_columns: vec![],
            force_fixed_patterns: vec![
                r"__[0-9]+__".to_string(),
                r"__[A-Z]+__[A-Za-z0-9_\-\.]+__".to_string(),
            ],
            forbidden_substitutions: vec![],
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
