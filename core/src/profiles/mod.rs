/// Game profile system for game-specific translation rules
pub mod rimworld;
pub mod factorio;
pub mod stardew;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProfile {
    pub id: String,
    pub name: String,
    pub detector: DetectionRules,
    pub include_paths: Vec<String>,
    pub exclude_paths: Vec<String>,
    pub extra_placeholders: Vec<String>,
    pub terminology: HashMap<String, String>,
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
        
        if stardew::StardewValleyProfile::detect(mod_path) {
            return Some(stardew::StardewValleyProfile::profile());
        }
        
        None
    }
    
    /// Get generic profile (fallback)
    pub fn generic() -> Self {
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
        }
    }
}

/// Get all available profiles
pub fn list_profiles() -> Vec<GameProfile> {
    vec![
        rimworld::RimWorldProfile::profile(),
        factorio::FactorioProfile::profile(),
        stardew::StardewValleyProfile::profile(),
        GameProfile::generic(),
    ]
}
