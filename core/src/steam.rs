use dirs::home_dir;
use dunce::canonicalize as dunce_canonicalize;
use log::{info, warn};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CanonicalizedPathSnapshot {
    pub original: String,
    pub canonical: Option<String>,
    pub key: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DuplicateLibrarySnapshot {
    pub existing: String,
    pub duplicate: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RejectedLibraryCandidate {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LibraryDiscoveryDebug {
    #[serde(default)]
    pub raw_candidates: Vec<String>,
    #[serde(default)]
    pub canonicalized: Vec<CanonicalizedPathSnapshot>,
    #[serde(default)]
    pub skipped_symlinks: Vec<String>,
    #[serde(default)]
    pub collapsed_duplicates: Vec<DuplicateLibrarySnapshot>,
    #[serde(default)]
    pub rejected_candidates: Vec<RejectedLibraryCandidate>,
    #[serde(default)]
    pub final_libraries: Vec<String>,
}

#[derive(Debug, Default)]
pub struct LibraryDiscovery {
    pub paths: Vec<PathBuf>,
    pub debug: LibraryDiscoveryDebug,
}

#[derive(Debug, Default)]
struct LibraryFoldersParseResult {
    paths: Vec<PathBuf>,
    rejections: Vec<RejectedLibraryCandidate>,
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

    pub fn library_candidates(&self, explicit: Option<&str>) -> LibraryDiscovery {
        let mut debug = LibraryDiscoveryDebug::default();
        let mut raw_paths: Vec<PathBuf> = Vec::new();

        if let Some(path) = explicit {
            push_raw(&mut raw_paths, &mut debug, PathBuf::from(path));
        }

        if let Some(primary) = self.discover_path() {
            push_raw(&mut raw_paths, &mut debug, primary.clone());
            let parsed = self.parse_library_folders(&primary);
            debug
                .rejected_candidates
                .extend(parsed.rejections.into_iter());
            for extra in parsed.paths {
                push_raw(&mut raw_paths, &mut debug, extra);
            }
        }

        for candidate in self.candidate_roots() {
            push_raw(&mut raw_paths, &mut debug, candidate);
        }

        if raw_paths.is_empty() {
            if let Some(home) = home_dir() {
                push_raw(&mut raw_paths, &mut debug, home.join(".steam"));
            }
        }

        let mut seen: HashMap<String, PathBuf> = HashMap::new();
        let mut final_paths = Vec::new();

        for candidate in raw_paths {
            let candidate_str = display_path(&candidate);

            let metadata = match fs::symlink_metadata(&candidate) {
                Ok(meta) => meta,
                Err(err) => {
                    debug.rejected_candidates.push(RejectedLibraryCandidate {
                        path: candidate_str.clone(),
                        reason: format!("파일 정보를 확인하지 못했습니다: {err}"),
                    });
                    debug.canonicalized.push(CanonicalizedPathSnapshot {
                        original: candidate_str,
                        canonical: None,
                        key: None,
                        status: "metadata_error".into(),
                        note: None,
                    });
                    continue;
                }
            };

            if metadata.file_type().is_symlink() {
                warn!("Skipped symlinked library root: {candidate_str}");
                debug.skipped_symlinks.push(candidate_str.clone());
                debug.rejected_candidates.push(RejectedLibraryCandidate {
                    path: candidate_str.clone(),
                    reason: "심볼릭 링크 또는 정션 경로를 건너뜁니다.".into(),
                });
                debug.canonicalized.push(CanonicalizedPathSnapshot {
                    original: candidate_str,
                    canonical: None,
                    key: None,
                    status: "symlink".into(),
                    note: Some("중복 탐지를 위해 심볼릭 링크를 무시했습니다.".into()),
                });
                continue;
            }

            if !metadata.is_dir() {
                debug.rejected_candidates.push(RejectedLibraryCandidate {
                    path: candidate_str.clone(),
                    reason: "디렉터리가 아니라서 제외했습니다.".into(),
                });
                debug.canonicalized.push(CanonicalizedPathSnapshot {
                    original: candidate_str,
                    canonical: None,
                    key: None,
                    status: "not_directory".into(),
                    note: None,
                });
                continue;
            }

            let canonical_path = match canonicalize_path(&candidate) {
                Ok(path) => path,
                Err(err) => {
                    let note = format!("경로를 정규화하지 못했습니다: {err}");
                    warn!("{note}");
                    debug.rejected_candidates.push(RejectedLibraryCandidate {
                        path: candidate_str.clone(),
                        reason: note.clone(),
                    });
                    debug.canonicalized.push(CanonicalizedPathSnapshot {
                        original: candidate_str,
                        canonical: None,
                        key: None,
                        status: "canonicalization_failed".into(),
                        note: Some(note),
                    });
                    continue;
                }
            };

            let key = path_dedupe_key(&canonical_path);
            let canonical_str = display_path(&canonical_path);

            if let Some(existing) = seen.get(&key) {
                let existing_str = display_path(existing);
                info!(
                    "Collapsed duplicate library: '{}' == '{}'",
                    existing_str, canonical_str
                );
                debug.collapsed_duplicates.push(DuplicateLibrarySnapshot {
                    existing: existing_str.clone(),
                    duplicate: candidate_str.clone(),
                });
                debug.canonicalized.push(CanonicalizedPathSnapshot {
                    original: candidate_str,
                    canonical: Some(canonical_str),
                    key: Some(key),
                    status: "duplicate".into(),
                    note: Some(format!("이미 포함된 경로와 동일: {existing_str}")),
                });
                continue;
            }

            let mut note = None;
            if !canonical_path.join("steamapps").is_dir() {
                note = Some("steamapps 디렉터리를 찾을 수 없어 누락 상태로 표시됩니다.".into());
            }

            debug.canonicalized.push(CanonicalizedPathSnapshot {
                original: candidate_str,
                canonical: Some(canonical_str.clone()),
                key: Some(key.clone()),
                status: "accepted".into(),
                note,
            });

            seen.insert(key, canonical_path.clone());
            final_paths.push(canonical_path);
        }

        debug.final_libraries = final_paths.iter().map(|path| display_path(path)).collect();

        LibraryDiscovery {
            paths: final_paths,
            debug,
        }
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

    fn parse_library_folders(&self, steam_root: &Path) -> LibraryFoldersParseResult {
        let mut result = LibraryFoldersParseResult::default();
        let library_vdf = steam_root.join("steamapps/libraryfolders.vdf");
        let contents = match fs::read_to_string(&library_vdf) {
            Ok(contents) => contents,
            Err(err) => {
                result.rejections.push(RejectedLibraryCandidate {
                    path: display_path(&library_vdf),
                    reason: format!("libraryfolders.vdf를 읽지 못했습니다: {err}"),
                });
                result.paths.push(steam_root.to_path_buf());
                return result;
            }
        };

        let mut seen = HashSet::new();
        for capture in LIBRARY_PATH_CAPTURE.captures_iter(&contents) {
            let raw_path = capture[1].replace("\\\\", "\\");
            let path = PathBuf::from(raw_path.clone());
            let path_str = display_path(&path);

            if !seen.insert(path_str.clone()) {
                continue;
            }

            if !path.exists() || !path.is_dir() {
                result.rejections.push(RejectedLibraryCandidate {
                    path: path_str,
                    reason: "경로가 존재하지 않거나 디렉터리가 아닙니다.".into(),
                });
                continue;
            }

            if !path.join("steamapps").is_dir() {
                result.rejections.push(RejectedLibraryCandidate {
                    path: path_str,
                    reason: "steamapps 디렉터리를 찾지 못했습니다.".into(),
                });
                continue;
            }

            result.paths.push(path);
        }

        if result.paths.is_empty() {
            result.paths.push(steam_root.to_path_buf());
        }

        result
    }
}

fn push_raw(raw_paths: &mut Vec<PathBuf>, debug: &mut LibraryDiscoveryDebug, path: PathBuf) {
    debug.raw_candidates.push(display_path(&path));
    raw_paths.push(path);
}

fn canonicalize_path(path: &Path) -> Result<PathBuf, io::Error> {
    dunce_canonicalize(path).or_else(|_| std::fs::canonicalize(path))
}

#[cfg(target_os = "windows")]
fn path_dedupe_key(path: &Path) -> String {
    let mut key = display_path(path).replace('/', "\\");
    if let Some(stripped) = key.strip_prefix(r"\\?\") {
        key = stripped.to_string();
    }
    if key.len() >= 2 && key.as_bytes()[1] == b':' {
        let mut chars: Vec<char> = key.chars().collect();
        if let Some(first) = chars.first_mut() {
            *first = first.to_ascii_lowercase();
        }
        key = chars.into_iter().collect();
    }
    while key.ends_with('\u{005c}') && key.len() > 3 {
        key.pop();
    }
    key
}

#[cfg(not(target_os = "windows"))]
fn path_dedupe_key(path: &Path) -> String {
    let mut key = display_path(path);
    if key.len() > 1 {
        key = key.trim_end_matches('/').to_string();
    }
    key
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
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
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

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
        let discovery = locator.library_candidates(None);
        assert!(!discovery.paths.is_empty());
        assert!(!discovery.debug.raw_candidates.is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn dedupe_key_normalizes_windows_variants() {
        let key_base = path_dedupe_key(Path::new(r"C:\SteamLibrary"));
        let key_lower = path_dedupe_key(Path::new(r"c:\SteamLibrary\"));
        // Test that both normalize to the same key
        assert_eq!(key_base, key_lower, "Base and lowercase should match: {} vs {}", key_base, key_lower);
        
        // UNC path normalization - just verify it produces a consistent key
        let key_unc = path_dedupe_key(Path::new(r"\\?\C:\SteamLibrary"));
        // The key should start with lowercase drive letter
        assert!(key_unc.starts_with("c:"), "UNC path key should start with c: but got {}", key_unc);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn dedupe_key_trims_trailing_separator() {
        let key_base = path_dedupe_key(Path::new("/tmp/mod-translator"));
        let key_with_slash = path_dedupe_key(Path::new("/tmp/mod-translator/"));
        assert_eq!(key_base, key_with_slash);
    }

    #[cfg(unix)]
    #[test]
    fn library_candidates_skip_symlink_roots() {
        let locator = SteamLocator::new();
        let temp_root =
            std::env::temp_dir().join(format!("mod_translator_symlink_test_{}", Uuid::new_v4()));
        let target = temp_root.join("library");
        fs::create_dir_all(target.join("steamapps")).expect("create target library");
        let link = temp_root.join("library_link");
        symlink(&target, &link).expect("create symlink");
        let link_str = link.to_string_lossy().to_string();

        let discovery = locator.library_candidates(Some(&link_str));
        assert!(discovery.paths.is_empty());
        assert!(discovery.debug.skipped_symlinks.contains(&link_str));

        fs::remove_dir_all(&temp_root).ok();
    }
}
