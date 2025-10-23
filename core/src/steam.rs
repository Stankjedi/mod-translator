use dirs::home_dir;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use winreg::enums::HKEY_CURRENT_USER;
#[cfg(target_os = "windows")]
use winreg::RegKey;

static LIBRARY_PATH_CAPTURE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"path"\s+"([^"]+)"#).expect("valid library path regex"));

#[derive(Debug, Default)]
pub struct SteamLocator;

impl SteamLocator {
    pub fn new() -> Self {
        Self
    }

    pub fn discover_path(&self) -> Option<PathBuf> {
        if let Some(path) = self.env_override() {
            return Some(path);
        }

        if let Some(path) = self.registry_install_path() {
            return Some(path);
        }

        self.candidate_roots().into_iter().find(|candidate| {
            candidate.join("steam.exe").exists() || candidate.join("Steam.app").exists()
        })
    }

    pub fn library_candidates(&self, explicit: Option<&str>) -> Vec<PathBuf> {
        let mut seen = HashSet::new();
        let mut results = Vec::new();

        let mut push_unique = |path: PathBuf, results: &mut Vec<PathBuf>| {
            let key = path.to_string_lossy().to_string();
            if seen.insert(key) {
                results.push(path);
            }
        };

        if let Some(path) = explicit {
            push_unique(PathBuf::from(path), &mut results);
        }

        if let Some(primary) = self.discover_path() {
            push_unique(primary.clone(), &mut results);
            for extra in self.parse_library_folders(&primary) {
                push_unique(extra, &mut results);
            }
        }

        for candidate in self.candidate_roots() {
            push_unique(candidate, &mut results);
        }

        if results.is_empty() {
            if let Some(home) = home_dir() {
                push_unique(home.join(".steam"), &mut results);
            }
        }

        results
    }

    pub fn app_manifests(&self, library_root: &Path) -> Vec<PathBuf> {
        let steamapps = library_root.join("steamapps");
        match fs::read_dir(&steamapps) {
            Ok(entries) => entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.starts_with("appmanifest_") && name.ends_with(".acf"))
                        .unwrap_or(false)
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn workshop_mappings(&self, library_root: &Path) -> Vec<PathBuf> {
        let workshop_root = library_root.join("steamapps/workshop");
        match fs::read_dir(&workshop_root) {
            Ok(entries) => entries.flatten().map(|entry| entry.path()).collect(),
            Err(_) => Vec::new(),
        }
    }

    fn env_override(&self) -> Option<PathBuf> {
        std::env::var("STEAM_PATH")
            .ok()
            .map(PathBuf::from)
            .filter(|path| path.exists())
    }

    #[cfg(target_os = "windows")]
    fn registry_install_path(&self) -> Option<PathBuf> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu.open_subkey("Software\\Valve\\Steam").ok()?;
        let path: String = key.get_value("SteamPath").ok()?;
        let path = PathBuf::from(path);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn registry_install_path(&self) -> Option<PathBuf> {
        None
    }

    fn candidate_roots(&self) -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        if let Some(home) = home_dir() {
            #[cfg(target_os = "windows")]
            {
                candidates.push(home.join("AppData/Local/Steam"));
                candidates.push(PathBuf::from("C:/Program Files (x86)/Steam"));
            }

            #[cfg(target_os = "linux")]
            {
                candidates.push(home.join(".steam/steam"));
                candidates.push(home.join(".local/share/Steam"));
            }

            #[cfg(target_os = "macos")]
            {
                candidates.push(home.join("Library/Application Support/Steam"));
            }
        }

        candidates
    }

    fn parse_library_folders(&self, steam_root: &Path) -> Vec<PathBuf> {
        let library_vdf = steam_root.join("steamapps/libraryfolders.vdf");
        let contents = match fs::read_to_string(&library_vdf) {
            Ok(contents) => contents,
            Err(_) => return vec![steam_root.to_path_buf()],
        };

        let mut libraries = Vec::new();
        for capture in LIBRARY_PATH_CAPTURE.captures_iter(&contents) {
            let raw_path = capture[1].replace("\\\\", "\\");
            let path = PathBuf::from(raw_path.clone());
            if path.exists() {
                libraries.push(path);
            }
        }

        if libraries.is_empty() {
            libraries.push(steam_root.to_path_buf());
        }

        libraries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_generation_provides_fallback() {
        let locator = SteamLocator::new();
        let candidates = locator.library_candidates(None);
        assert!(!candidates.is_empty());
    }
}
