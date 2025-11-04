/// Stardew Valley game profile
use super::{DetectionRules, GameProfile};
use std::path::Path;
use std::collections::HashMap;

pub struct StardewValleyProfile;

impl StardewValleyProfile {
    pub fn detect(mod_path: &Path) -> bool {
        // Check for manifest.json (SMAPI mod)
        let manifest = mod_path.join("manifest.json");
        if manifest.exists() {
            return true;
        }
        
        // Check for i18n/ directory
        let i18n = mod_path.join("i18n");
        if i18n.exists() && i18n.is_dir() {
            return true;
        }
        
        false
    }
    
    pub fn profile() -> GameProfile {
        GameProfile {
            id: "stardew".to_string(),
            name: "Stardew Valley".to_string(),
            detector: DetectionRules {
                folder_patterns: vec!["i18n/".to_string()],
                file_patterns: vec!["manifest.json".to_string()],
                manifest_signatures: vec!["ContentPackFor".to_string()],
            },
            include_paths: vec!["i18n/".to_string()],
            exclude_paths: vec![
                "assets/".to_string(),
            ],
            extra_placeholders: vec![
                r"\{0\}".to_string(),
                r"\{[a-zA-Z]+\}".to_string(),
            ],
            terminology: HashMap::new(),
        }
    }
}
