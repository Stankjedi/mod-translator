/// RimWorld game profile
use super::{DetectionRules, GameProfile};
use std::path::Path;
use std::collections::HashMap;

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
        }
    }
}
