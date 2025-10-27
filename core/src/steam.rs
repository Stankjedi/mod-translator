use dirs::home_dir;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use winreg::enums::HKEY_CURRENT_USER;
#[cfg(target_os = "windows")]
use winreg::RegKey;

static LIBRARY_PATH_CAPTURE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"path"\s+"([^"]+)"#).expect("valid library path regex"));
static APP_NAME_CAPTURE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?m)"name"\s+"([^"]+)""#).expect("valid Steam app name regex"));

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SteamPathResponse {
    pub path: Option<String>,
    pub note: String,
}

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
        let mut seen: HashSet<PathBuf> = HashSet::new();
        let mut results = Vec::new();

        let mut push_unique = |path: PathBuf, results: &mut Vec<PathBuf>| {
            if seen.insert(path.clone()) {
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
                if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
                    candidates.push(PathBuf::from(program_files_x86).join("Steam"));
                }
                if let Some(program_files) = std::env::var_os("ProgramW6432") {
                    candidates.push(PathBuf::from(program_files).join("Steam"));
                }
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

pub fn resolve_app_name(steamapps: &Path, app_id: &str) -> Option<String> {
    let manifest_path = steamapps.join(format!("appmanifest_{}.acf", app_id));
    let contents = fs::read_to_string(manifest_path).ok()?;
    APP_NAME_CAPTURE
        .captures(&contents)
        .and_then(|capture| capture.get(1))
        .map(|name| name.as_str().trim().to_string())
        .filter(|name| !name.is_empty())
}

#[tauri::command]
pub fn detect_steam_path() -> Result<SteamPathResponse, String> {
    let locator = SteamLocator::new();
    let discovered = locator.discover_path();

    let note = if discovered.is_some() {
        "Steam 설치 경로를 자동으로 감지했습니다.".to_string()
    } else {
        "Steam 경로를 자동으로 찾지 못했습니다. 직접 경로를 입력해 주세요.".to_string()
    };

    let path = match discovered {
        Some(path_buf) => Some(
            path_buf
                .to_str()
                .ok_or_else(|| "Steam 경로를 UTF-8로 변환하지 못했습니다.".to_string())?
                .to_string(),
        ),
        None => None,
    };

    Ok(SteamPathResponse { path, note })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn create_temp_steamapps(manifest_contents: &str, app_id: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mod_translator_resolve_app_name_test_{}",
            Uuid::new_v4()
        ));
        let steamapps = root.join("steamapps");
        fs::create_dir_all(&steamapps).expect("create steamapps temp dir");
        let manifest_path = steamapps.join(format!("appmanifest_{}.acf", app_id));
        fs::write(manifest_path, manifest_contents).expect("write manifest contents");
        steamapps
    }

    #[test]
    fn extracts_app_name_from_manifest() {
        let contents = r#"
"AppState"
{
    "appid"        "294100"
    "Universe"     "1"
    "name"         "RimWorld"
}
"#;
        let steamapps = create_temp_steamapps(contents, "294100");
        let resolved = resolve_app_name(&steamapps, "294100");
        assert_eq!(resolved.as_deref(), Some("RimWorld"));
        fs::remove_dir_all(steamapps.parent().unwrap()).ok();
    }

    #[test]
    fn returns_none_when_manifest_missing() {
        let steamapps = create_temp_steamapps("", "111111");
        let resolved = resolve_app_name(&steamapps, "222222");
        assert!(resolved.is_none());
        fs::remove_dir_all(steamapps.parent().unwrap()).ok();
    }

    #[test]
    fn candidate_generation_provides_fallback() {
        let locator = SteamLocator::new();
        let candidates = locator.library_candidates(None);
        assert!(!candidates.is_empty());
    }
}
