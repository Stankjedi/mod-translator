/// RimWorld game profile
use super::{DetectionRules, GameProfile, ValidatorProfileConfig, FormatRule, TokenSubstitution};
use std::path::Path;
use std::collections::{HashMap, HashSet};

pub struct RimWorldProfile;

impl RimWorldProfile {
    pub fn detect(mod_path: &Path) -> bool {
        // Check for About/About.xml
        let about_xml = mod_path.join("About").join("About.xml");
        if about_xml.exists() {
            return true;
        }
        
        // Check for Languages/ directory
        let languages = mod_path.join("Languages");
        if languages.exists() && languages.is_dir() {
            return true;
        }
        
        false
    }
    
    pub fn profile() -> GameProfile {
        let mut terminology = HashMap::new();
        terminology.insert("pawn".to_string(), "폰".to_string());
        terminology.insert("colonist".to_string(), "정착민".to_string());
        
        // Validator configuration for RimWorld (Section 3)
        let mut allowed_token_types = HashSet::new();
        allowed_token_types.insert("DOTNET".to_string());      // {0}, {1}
        allowed_token_types.insert("NAMED".to_string());       // {PAWN_label}, {name}
        allowed_token_types.insert("TAG".to_string());         // <tag>
        allowed_token_types.insert("RWCOLOR".to_string());     // <color=#fff>
        allowed_token_types.insert("RICHTEXT".to_string());    // <sprite>
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
                // PAWN tokens have fixed spelling
                r"\{PAWN_[A-Za-z_]+\}".to_string(),
                // {n}% patterns are atomic
                r"\{[0-9]+\}%".to_string(),
                // Mathematical expressions
                r"\d+(?:\.\d+)?\s*[+\-×*÷/^=≠≈≤≥<>]\s*\d+".to_string(),
            ],
            forbidden_substitutions: vec![
                TokenSubstitution {
                    from: "%s".to_string(),
                    to: "{0}".to_string(),
                    reason: "RimWorld uses .NET format, not printf format".to_string(),
                },
            ],
            format_rules: vec![
                FormatRule {
                    format: "xml".to_string(),
                    rule_type: "nested_color_tags".to_string(),
                    description: "Allow nested <color> tags with auto-balancing".to_string(),
                },
                FormatRule {
                    format: "xml".to_string(),
                    rule_type: "percent_binding".to_string(),
                    description: "{0}% patterns must remain together".to_string(),
                },
            ],
        };
        
        GameProfile {
            id: "rimworld".to_string(),
            name: "RimWorld".to_string(),
            detector: DetectionRules {
                folder_patterns: vec!["Languages/".to_string(), "About/".to_string()],
                file_patterns: vec!["About.xml".to_string()],
                manifest_signatures: vec!["<ModMetaData>".to_string()],
            },
            include_paths: vec![
                "Languages/".to_string(),
                "Defs/".to_string(),
            ],
            exclude_paths: vec![
                "Assemblies/".to_string(),
                "Textures/".to_string(),
            ],
            extra_placeholders: vec![
                r"\{PAWN_[^}]+\}".to_string(),
                r"\{[A-Z_]+\}".to_string(),
            ],
            terminology,
            validator_config,
        }
    }
}
