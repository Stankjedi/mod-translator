/// Minecraft game profile
use super::{DetectionRules, GameProfile, ValidatorProfileConfig, FormatRule, TokenSubstitution};
use std::path::Path;
use std::collections::{HashMap, HashSet};

pub struct MinecraftProfile;

impl MinecraftProfile {
    pub fn detect(mod_path: &Path) -> bool {
        // Check for mcmod.info (Minecraft Forge mod manifest)
        let mcmod_info = mod_path.join("mcmod.info");
        if mcmod_info.exists() {
            return true;
        }
        
        // Check for fabric.mod.json (Fabric mod manifest)
        let fabric_json = mod_path.join("fabric.mod.json");
        if fabric_json.exists() {
            return true;
        }
        
        // Check for assets/ directory
        let assets = mod_path.join("assets");
        if assets.exists() && assets.is_dir() {
            return true;
        }
        
        false
    }
    
    pub fn profile() -> GameProfile {
        // Validator configuration for Minecraft (Section 3)
        let mut allowed_token_types = HashSet::new();
        allowed_token_types.insert("PRINTF".to_string());      // %s, %d, %1$s
        allowed_token_types.insert("NAMED".to_string());       // {name}
        allowed_token_types.insert("MCCOLOR".to_string());     // §a, §l
        allowed_token_types.insert("ENTITY".to_string());      // &nbsp;
        allowed_token_types.insert("PERCENT".to_string());     // percentages
        allowed_token_types.insert("UNIT".to_string());        // units with numbers
        allowed_token_types.insert("MATHEXPR".to_string());    // math expressions
        allowed_token_types.insert("RANGE".to_string());       // ranges
        allowed_token_types.insert("SCIENTIFIC".to_string());  // scientific notation
        
        let validator_config = ValidatorProfileConfig {
            allowed_token_types,
            csv_target_columns: vec![],
            force_fixed_patterns: vec![
                // Simple printf: %s, %d
                r"%[ds]".to_string(),
                // Positional printf: %1$s
                r"%[0-9]+\$[ds]".to_string(),
                // Color codes: §a, §l, §r
                r"§[0-9A-FK-ORa-fk-or]".to_string(),
            ],
            forbidden_substitutions: vec![
                TokenSubstitution {
                    from: "%s".to_string(),
                    to: "{0}".to_string(),
                    reason: "Cannot convert printf to .NET format in Minecraft".to_string(),
                },
                TokenSubstitution {
                    from: "{0}".to_string(),
                    to: "%s".to_string(),
                    reason: "Cannot convert .NET format to printf in Minecraft".to_string(),
                },
            ],
            format_rules: vec![
                FormatRule {
                    format: "json".to_string(),
                    rule_type: "printf_type_preservation".to_string(),
                    description: "Cannot mix %s with %d or change types".to_string(),
                },
                FormatRule {
                    format: "json".to_string(),
                    rule_type: "color_at_edges".to_string(),
                    description: "§ codes must stay at text boundaries, not in middle".to_string(),
                },
                FormatRule {
                    format: "json".to_string(),
                    rule_type: "format_consistency".to_string(),
                    description: "Don't convert between printf and brace formats".to_string(),
                },
            ],
        };
        
        GameProfile {
            id: "minecraft".to_string(),
            name: "Minecraft".to_string(),
            detector: DetectionRules {
                folder_patterns: vec!["assets/".to_string(), "lang/".to_string()],
                file_patterns: vec!["mcmod.info".to_string(), "fabric.mod.json".to_string()],
                manifest_signatures: vec!["\"modid\"".to_string()],
            },
            include_paths: vec![
                "assets/".to_string(),
                "lang/".to_string(),
            ],
            exclude_paths: vec![
                "textures/".to_string(),
                "models/".to_string(),
                "sounds/".to_string(),
            ],
            extra_placeholders: vec![
                r"%\d*\$?[sdf]".to_string(),
                r"§[0-9A-FK-ORa-fk-or]".to_string(),
            ],
            terminology: HashMap::new(),
            validator_config,
        }
    }
}
