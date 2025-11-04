/// Factorio game profile
use super::{DetectionRules, GameProfile};
use std::path::Path;
use std::collections::HashMap;

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
        }
    }
}
