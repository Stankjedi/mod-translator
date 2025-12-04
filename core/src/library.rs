use crate::archive::{self, ArchiveType};
use crate::policy::{self, PolicyBanner, PolicyProfile};
use crate::steam::{resolve_app_name, LibraryDiscovery, LibraryDiscoveryDebug, SteamLocator};
use crate::time::{format_system_time, FormattedTimestamp};
use log::{info, warn};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

static RIMWORLD_ABOUT_NAME_CAPTURE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)<name>\s*([^<]+?)\s*</name>").expect("valid RimWorld About.xml name regex")
});

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LibraryStatus {
    Healthy,
    Missing,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModSummary {
    pub id: String,
    pub name: String,
    pub game: String,
    pub directory: String,
    pub installed_languages: Vec<String>,
    pub last_updated: FormattedTimestamp,
    pub policy: PolicyProfile,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModFileDescriptor {
    pub path: String,
    pub mod_install_path: String,
    pub translatable: bool,
    #[serde(default)]
    pub auto_selected: bool,
    pub language_hint: Option<String>,
    /// 아카이브 내부 파일인 경우 아카이브 경로
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_path: Option<String>,
    /// 아카이브 타입 (jar, zip)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_type: Option<ArchiveType>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModFileListing {
    pub files: Vec<ModFileDescriptor>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LibraryWorkshopDebugEntry {
    pub library: String,
    pub total_candidates: usize,
    pub unique_mods: usize,
    #[serde(default)]
    pub duplicates: Vec<String>,
    #[serde(default)]
    pub skipped_symlinks: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryScanDebug {
    pub discovery: LibraryDiscoveryDebug,
    #[serde(default)]
    pub workshop: Vec<LibraryWorkshopDebugEntry>,
}

impl LibraryScanDebug {
    pub fn new(discovery: LibraryDiscoveryDebug) -> Self {
        Self {
            discovery,
            workshop: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryEntry {
    pub path: String,
    pub status: LibraryStatus,
    pub mods: Vec<ModSummary>,
    pub workshop_root: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryScanResponse {
    pub libraries: Vec<LibraryEntry>,
    pub policy_banner: PolicyBanner,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<LibraryScanDebug>,
}

#[derive(Debug, Default)]
pub struct LibraryScanner;

impl LibraryScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan(
        &self,
        candidates: &[PathBuf],
        debug: &mut LibraryScanDebug,
    ) -> Result<Vec<LibraryEntry>, String> {
        if candidates.is_empty() {
            return Ok(vec![
                self.placeholder_entry("라이브러리 경로를 찾을 수 없음")
            ]);
        }

        let mut entries = Vec::new();
        let mut global_mods: HashSet<String> = HashSet::new();

        for root in candidates {
            let steamapps = root.join("steamapps");
            let exists = steamapps.exists();
            let workshop_root = steamapps.join("workshop");
            let mut workshop_display = None;
            let mut notes = Vec::new();
            let mut workshop_debug = LibraryWorkshopDebugEntry {
                library: root.to_string_lossy().to_string(),
                ..Default::default()
            };

            let mods = if exists {
                match self.detect_workshop_mods(&steamapps, &mut global_mods, &mut workshop_debug) {
                    Ok(result) => result,
                    Err(err) => {
                        notes.push(err);
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            };

            if !exists {
                notes.push(
                    "steamapps 디렉터리를 찾을 수 없습니다. 설정에서 라이브러리를 수동으로 추가하세요.".into(),
                );
            } else if mods.is_empty() {
                if workshop_debug.total_candidates == 0 {
                    notes.push(
                        "워크샵 콘텐츠를 찾지 못했습니다. Steam을 한 번 실행하여 appworkshop 정보를 생성하세요.".into(),
                    );
                }
            }

            if !workshop_debug.duplicates.is_empty() {
                let preview: Vec<_> = workshop_debug.duplicates.iter().take(3).cloned().collect();
                let preview_text = if preview.is_empty() {
                    String::new()
                } else {
                    let suffix = if workshop_debug.duplicates.len() > preview.len() {
                        ", ... 포함".to_string()
                    } else {
                        String::new()
                    };
                    format!(" (예: {}{})", preview.join(", "), suffix)
                };
                notes.push(format!(
                    "다른 라이브러리와 중복된 워크샵 항목 {}개를 건너뛰었습니다{}.",
                    workshop_debug.duplicates.len(),
                    preview_text
                ));
            }

            if !workshop_debug.skipped_symlinks.is_empty() {
                notes.push(format!(
                    "심볼릭 링크로 연결된 워크샵 경로 {}개를 건너뛰었습니다.",
                    workshop_debug.skipped_symlinks.len()
                ));
            }

            if workshop_root.exists() {
                match to_utf8_string(&workshop_root) {
                    Ok(path) => workshop_display = Some(path),
                    Err(err) => notes.push(err),
                }
            }

            let entry_path = match to_utf8_string(root) {
                Ok(path) => path,
                Err(err) => return Err(err),
            };

            workshop_debug.library = entry_path.clone();
            debug.workshop.push(workshop_debug);

            entries.push(LibraryEntry {
                path: entry_path,
                status: if exists {
                    LibraryStatus::Healthy
                } else {
                    LibraryStatus::Missing
                },
                mods,
                workshop_root: workshop_display,
                notes,
            });
        }

        Ok(entries)
    }

    fn placeholder_entry(&self, label: &str) -> LibraryEntry {
        LibraryEntry {
            path: label.to_string(),
            status: LibraryStatus::Missing,
            mods: vec![],
            workshop_root: None,
            notes: vec!["Steam 라이브러리 경로를 입력하면 스캔을 시작할 수 있습니다.".into()],
        }
    }

    fn detect_workshop_mods(
        &self,
        steamapps: &Path,
        global_unique: &mut HashSet<String>,
        workshop_debug: &mut LibraryWorkshopDebugEntry,
    ) -> Result<Vec<ModSummary>, String> {
        let mut mods = Vec::new();
        let content_root = steamapps.join("workshop/content");
        if !content_root.exists() {
            return Ok(mods);
        }

        let app_dirs = match fs::read_dir(&content_root) {
            Ok(entries) => entries,
            Err(err) => {
                return Err(format!("워크샵 콘텐츠를 열거하지 못했습니다: {err}"));
            }
        };

        for app_dir in app_dirs.flatten() {
            let app_path = app_dir.path();
            let Ok(app_meta) = fs::symlink_metadata(&app_path) else {
                continue;
            };

            if app_meta.file_type().is_symlink() {
                let path_display = app_path.to_string_lossy().to_string();
                workshop_debug.skipped_symlinks.push(path_display.clone());
                warn!("Skipped symlinked workshop directory: {path_display}");
                continue;
            }

            if !app_meta.is_dir() {
                continue;
            }

            let app_id = match file_name_to_string(&app_path) {
                Ok(value) => value,
                Err(err) => {
                    mods.push(self.synthetic_mod(
                        "invalid-app-name",
                        "워크샵 콘텐츠 식별자를 해석할 수 없습니다",
                        "알 수 없는 게임".to_string(),
                        vec!["en".into()],
                        err,
                    ));
                    continue;
                }
            };

            let fallback_game = format!("앱 {app_id}");
            let game_name = resolve_app_name(steamapps, &app_id).unwrap_or(fallback_game);

            let mod_entries = match fs::read_dir(&app_path) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for mod_dir in mod_entries.flatten() {
                let mod_path = mod_dir.path();
                let Ok(mod_meta) = fs::symlink_metadata(&mod_path) else {
                    continue;
                };

                if !mod_meta.is_dir() {
                    continue;
                }

                workshop_debug.total_candidates += 1;

                if mod_meta.file_type().is_symlink() {
                    let path_display = mod_path.to_string_lossy().to_string();
                    workshop_debug.skipped_symlinks.push(path_display.clone());
                    warn!("Skipped symlinked workshop item: {path_display}");
                    continue;
                }

                let mod_id = match file_name_to_string(&mod_path) {
                    Ok(value) => value,
                    Err(err) => {
                        mods.push(self.synthetic_mod(
                            "invalid-mod-name",
                            "모드 폴더 이름을 해석할 수 없습니다",
                            game_name.clone(),
                            vec!["en".into()],
                            err,
                        ));
                        continue;
                    }
                };

                let dedupe_key = format!("{app_id}:{mod_id}");
                if !global_unique.insert(dedupe_key.clone()) {
                    workshop_debug.duplicates.push(dedupe_key.clone());
                    info!("Skipped duplicate workshop item {dedupe_key}");
                    continue;
                }

                workshop_debug.unique_mods += 1;

                let metadata = fs::metadata(&mod_path).ok();
                let last_updated = metadata
                    .and_then(|meta| meta.modified().ok())
                    .and_then(|time| format_system_time(time).ok())
                    .unwrap_or_else(|| FormattedTimestamp::new("알 수 없음".into()));

                let languages = self.detect_languages(&mod_path);
                let warnings = self.collect_warnings(&mod_path);
                let resolved_name = self
                    .resolve_mod_name(&steamapps, &app_id, &mod_id, &mod_path)
                    .unwrap_or_else(|| format!("워크샵 항목 {mod_id}"));

                let directory = match to_utf8_string(&mod_path) {
                    Ok(value) => value,
                    Err(err) => {
                        mods.push(self.synthetic_mod(
                            "invalid-path",
                            "모드 경로를 UTF-8로 변환하지 못했습니다",
                            game_name.clone(),
                            vec!["en".into()],
                            err,
                        ));
                        continue;
                    }
                };

                mods.push(ModSummary {
                    id: dedupe_key,
                    name: resolved_name,
                    game: game_name.clone(),
                    directory,
                    installed_languages: languages,
                    last_updated,
                    policy: PolicyProfile::conservative(game_name.clone()),
                    warnings,
                });
            }
        }

        if mods.is_empty() && workshop_debug.total_candidates == 0 {
            mods.push(self.synthetic_mod(
                "no-content",
                "워크샵 아카이브를 찾지 못했습니다",
                "알 수 없는 게임".to_string(),
                vec!["en".into()],
                "Steam 동기화를 실행하여 workshop/content를 채워 주세요.".into(),
            ));
        }

        Ok(mods)
    }

    fn synthetic_mod(
        &self,
        mod_id: &str,
        name: &str,
        game_name: String,
        languages: Vec<String>,
        warning: String,
    ) -> ModSummary {
        ModSummary {
            id: format!("{}-{}", mod_id, Uuid::new_v4()),
            name: name.into(),
            game: game_name.clone(),
            directory: String::new(),
            installed_languages: languages,
            last_updated: FormattedTimestamp::new("알 수 없음".into()),
            policy: PolicyProfile::conservative(game_name),
            warnings: vec![warning],
        }
    }

    fn detect_languages(&self, path: &Path) -> Vec<String> {
        let mut detected = vec!["en".into()];
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let Some(name) = entry.file_name().to_str().map(|value| value.to_lowercase())
                else {
                    continue;
                };
                if name.contains(".ko") || name.contains("korean") {
                    detected.push("ko".into());
                }
                if name.contains(".jp") || name.contains("japanese") {
                    detected.push("ja".into());
                }
                if name.contains(".ru") || name.contains("russian") {
                    detected.push("ru".into());
                }
            }
        }

        detected.sort();
        detected.dedup();
        detected
    }

    fn collect_warnings(&self, path: &Path) -> Vec<String> {
        let mut warnings = Vec::new();
        let mut contains_dll = false;
        let mut contains_binary = false;

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let Some(name) = entry.file_name().to_str().map(|value| value.to_lowercase())
                else {
                    warnings.push(
                        "파일 이름을 UTF-8로 변환하지 못했습니다. 경고 감지를 건너뜁니다.".into(),
                    );
                    continue;
                };
                if name.ends_with(".dll") {
                    contains_dll = true;
                }
                if name.ends_with(".assetbundle") || name.ends_with(".unity3d") {
                    contains_binary = true;
                }
            }
        }

        if contains_dll {
            warnings.push(
                "관리형 DLL이 감지되었습니다. 디컴파일 대신 리소스 추출 방식을 사용하세요.".into(),
            );
        }
        if contains_binary {
            warnings.push(
                "Unity AssetBundle이 감지되었습니다. 워크스페이스 정책에 따라 기본적으로 건너뜁니다.".into(),
            );
        }

        warnings
    }

    fn resolve_mod_name(
        &self,
        steamapps: &Path,
        app_id: &str,
        mod_id: &str,
        mod_path: &Path,
    ) -> Option<String> {
        resolve_workshop_title(steamapps, app_id, mod_id)
            .or_else(|| resolve_about_metadata_title(mod_path))
    }
}

#[tauri::command]
pub fn list_mod_files(mod_directory: String) -> Result<ModFileListing, String> {
    let root = PathBuf::from(&mod_directory);
    if !root.exists() {
        return Err("모드 디렉터리를 찾을 수 없습니다.".into());
    }
    if !root.is_dir() {
        return Err("지정된 경로가 디렉터리가 아닙니다.".into());
    }

    let canonical_root = root
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&mod_directory));
    let mod_install_path = canonical_root
        .to_str()
        .map(|value| value.to_string())
        .ok_or_else(|| {
            format!(
                "경로를 UTF-8 문자열로 변환하지 못했습니다: {:?}",
                canonical_root
            )
        })?;

    let mut queue = VecDeque::new();
    queue.push_back(root.clone());
    let mut files = Vec::new();

    while let Some(dir) = queue.pop_front() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) => {
                return Err(format!("디렉터리를 열거하지 못했습니다: {err}"));
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                queue.push_back(path);
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            // JAR/ZIP 아카이브 파일인 경우 내부 스캔
            if archive::is_archive_file(&path) {
                if let Ok(scan_result) = archive::scan_archive(&path) {
                    let archive_rel = path.strip_prefix(&root)
                        .map(|p| normalize_relative_path(p))
                        .unwrap_or_else(|_| path.to_string_lossy().to_string());
                    
                    for entry in scan_result.language_files {
                        let language_hint = detect_archive_entry_language(&entry.path);
                        let auto_selected = language_hint.as_deref().map_or(true, |lang| lang != "ko");
                        
                        files.push(ModFileDescriptor {
                            path: entry.path,
                            mod_install_path: mod_install_path.clone(),
                            translatable: true,
                            auto_selected,
                            language_hint,
                            archive_path: Some(archive_rel.clone()),
                            archive_type: Some(scan_result.archive_type),
                        });
                    }
                }
                continue;
            }

            if let Some(descriptor) = classify_mod_file(&root, &path, &mod_install_path) {
                files.push(descriptor);
            }
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(ModFileListing { files })
}

#[tauri::command]
pub fn scan_steam_library(explicit_path: Option<String>) -> Result<LibraryScanResponse, String> {
    let locator = SteamLocator::new();
    let scanner = LibraryScanner::new();

    let primary_path = explicit_path
        .and_then(|value| {
            if value.trim().is_empty() {
                None
            } else {
                Some(value.into())
            }
        })
        .or_else(|| {
            locator
                .discover_path()
                .and_then(|path| to_utf8_string(&path).ok())
        });

    let LibraryDiscovery {
        paths: candidates,
        debug: discovery_debug,
    } = locator.library_candidates(primary_path.as_deref());
    let mut debug = LibraryScanDebug::new(discovery_debug);
    let libraries = scanner.scan(&candidates, &mut debug)?;

    Ok(LibraryScanResponse {
        libraries,
        policy_banner: policy::default_policy_banner(),
        debug: Some(debug),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn create_library_with_mod(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mod_translator_library_test_{}_{}",
            tag,
            Uuid::new_v4()
        ));
        let mod_dir = root.join("steamapps/workshop/content/294100/1234567890");
        fs::create_dir_all(&mod_dir).expect("create workshop mod directory");
        fs::write(mod_dir.join("About.xml"), "<name>Test</name>").ok();
        root
    }

    #[test]
    fn workshop_duplicates_are_deduped_across_libraries() {
        let lib_a = create_library_with_mod("a");
        let lib_b = create_library_with_mod("b");

        let scanner = LibraryScanner::new();
        let candidates = vec![lib_a.clone(), lib_b.clone()];
        let mut debug = LibraryScanDebug::new(LibraryDiscoveryDebug::default());
        let entries = scanner
            .scan(&candidates, &mut debug)
            .expect("scan libraries");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].mods.len(), 1);
        assert!(entries[1].mods.is_empty());
        assert!(entries[1].notes.iter().any(|note| note.contains("중복")));

        let total_unique: usize = debug.workshop.iter().map(|entry| entry.unique_mods).sum();
        assert_eq!(total_unique, 1);

        fs::remove_dir_all(&lib_a).ok();
        fs::remove_dir_all(&lib_b).ok();
    }

    fn temp_dir(label: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("mod_translator_{}_{}", label, Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn resolves_title_from_workshop_acf() {
        let root = temp_dir("workshop");
        let steamapps = root.join("steamapps");
        let workshop = steamapps.join("workshop");
        fs::create_dir_all(&workshop).expect("create workshop dir");
        let contents = r#"
"AppWorkshop"
{
    "appid"        "294100"
    "WorkshopItemDetails"
    {
        "123456"
        {
            "manifest"     "987654321"
            "timeupdated"  "1700000000"
            "title"        "Test Workshop Title"
        }
    }
}
"#;
        let file_path = workshop.join("appworkshop_294100.acf");
        fs::write(&file_path, contents).expect("write appworkshop file");

        let resolved = resolve_workshop_title(&steamapps, "294100", "123456");
        assert_eq!(resolved.as_deref(), Some("Test Workshop Title"));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn resolves_title_from_about_metadata() {
        let root = temp_dir("about");
        let mod_path = root.join("RimWorld");
        let about_dir = mod_path.join("About");
        fs::create_dir_all(&about_dir).expect("create about dir");
        let contents = r#"
<?xml version="1.0" encoding="utf-8"?>
<ModMetaData>
  <name>
    RimWorld Korean Language Pack
  </name>
</ModMetaData>
"#;
        fs::write(about_dir.join("About.xml"), contents).expect("write About.xml");

        let resolved = resolve_about_metadata_title(&mod_path);
        assert_eq!(resolved.as_deref(), Some("RimWorld Korean Language Pack"));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn prefers_workshop_title_over_about_metadata() {
        let root = temp_dir("prefer_workshop");
        let steamapps = root.join("steamapps");
        let workshop = steamapps.join("workshop");
        let mod_path = root.join("mod");
        fs::create_dir_all(&workshop).expect("create workshop dir");
        fs::create_dir_all(mod_path.join("About")).expect("create About dir");
        fs::write(
            workshop.join("appworkshop_123.acf"),
            r#"
"AppWorkshop"
{
    "appid"        "123"
    "WorkshopItemDetails"
    {
        "999"
        {
            "title" "Workshop Primary Title"
        }
    }
}
"#,
        )
        .expect("write workshop acf");
        fs::write(
            mod_path.join("About/About.xml"),
            r#"<ModMetaData><name>Local About Name</name></ModMetaData>"#,
        )
        .expect("write about xml");

        let scanner = LibraryScanner::new();
        let resolved = scanner.resolve_mod_name(&steamapps, "123", "999", &mod_path);
        assert_eq!(resolved.as_deref(), Some("Workshop Primary Title"));

        fs::remove_dir_all(root).ok();
    }
}


fn to_utf8_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(|value| value.to_string())
        .ok_or_else(|| format!("경로를 UTF-8 문자열로 변환하지 못했습니다: {:?}", path))
}

fn classify_mod_file(
    root: &Path,
    path: &Path,
    mod_install_path: &str,
) -> Option<ModFileDescriptor> {
    let relative = path.strip_prefix(root).ok()?;
    let relative_str = normalize_relative_path(relative);
    let lowered = relative_str.to_lowercase();

    if should_ignore_file(&lowered) {
        return None;
    }

    let segments: Vec<&str> = relative_str.split('/').collect();
    let directory_segments = if segments.is_empty() {
        Vec::new()
    } else {
        segments[..segments.len().saturating_sub(1)].to_vec()
    };

    let in_localization_dir = directory_segments
        .iter()
        .any(|segment| is_localization_directory(segment));

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());

    let language_hint = detect_language_hint(path);

    let is_text_extension = extension
        .as_deref()
        .map_or(false, |ext| matches_text_extension(ext));

    let translatable = is_text_extension || (in_localization_dir && language_hint.is_some());
    if !translatable {
        return None;
    }

    let auto_selected = if let Some(lang) = language_hint.as_deref() {
        lang != "ko"
    } else {
        in_localization_dir
    };

    Some(ModFileDescriptor {
        path: relative_str,
        mod_install_path: mod_install_path.to_string(),
        translatable: true,
        auto_selected,
        language_hint,
        archive_path: None,
        archive_type: None,
    })
}

fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn should_ignore_file(path: &str) -> bool {
    const BINARY_EXTENSIONS: &[&str] = &[
        "dll",
        "exe",
        "png",
        "jpg",
        "jpeg",
        "dds",
        "tga",
        "wav",
        "mp3",
        "ogg",
        "bank",
        "unity3d",
        "assetbundle",
        "bundle",
        "pck",
        "pak",
        "zip",
        "7z",
        "rar",
        "psd",
        "mp4",
        "mov",
    ];

    if let Some(ext) = path.rsplit('.').next() {
        if BINARY_EXTENSIONS.contains(&ext) {
            return true;
        }
    }

    path.contains("__macosx")
}

fn matches_text_extension(ext: &str) -> bool {
    const TEXT_EXTENSIONS: &[&str] = &[
        "txt",
        "cfg",
        "ini",
        "xml",
        "json",
        "yml",
        "yaml",
        "csv",
        "po",
        "pot",
        "resx",
        "resw",
        "strings",
        "properties",
        "loc",
        "lua",
        "md",
        "html",
        "htm",
        "dat",
        "defs",
    ];

    TEXT_EXTENSIONS.contains(&ext)
}

fn is_localization_directory(segment: &str) -> bool {
    matches!(
        segment.to_lowercase().as_str(),
        "localization"
            | "localisation"
            | "languages"
            | "language"
            | "lang"
            | "l10n"
            | "locale"
            | "loc"
            | "strings"
            | "text"
    )
}

fn detect_language_hint(path: &Path) -> Option<String> {
    let mut tokens = Vec::new();

    for component in path.components() {
        let part = component.as_os_str().to_string_lossy().to_lowercase();
        tokens.extend(split_language_tokens(&part));
    }

    for token in tokens {
        if let Some(code) = normalize_language_code(&token) {
            return Some(code);
        }
    }

    None
}

fn split_language_tokens(segment: &str) -> Vec<String> {
    segment
        .split(|c: char| c == '.' || c == '_' || c == '-' || c == ' ')
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect()
}

fn normalize_language_code(token: &str) -> Option<String> {
    match token {
        "en" | "eng" | "english" => Some("en".into()),
        "cn" | "chinese" | "chs" | "zh" | "zhcn" | "zh_hans" | "zh-hans" => Some("zh-cn".into()),
        "cht" | "zh_tw" | "zh-tw" | "traditionalchinese" => Some("zh-tw".into()),
        "jp" | "jpn" | "ja" | "japanese" => Some("ja".into()),
        "ru" | "rus" | "russian" => Some("ru".into()),
        "fr" | "fra" | "french" => Some("fr".into()),
        "de" | "ger" | "german" => Some("de".into()),
        "es" | "spa" | "spanish" => Some("es".into()),
        "pt" | "por" | "portuguese" => Some("pt".into()),
        "pl" | "pol" | "polish" => Some("pl".into()),
        "it" | "ita" | "italian" => Some("it".into()),
        "ko" | "kor" | "korean" => Some("ko".into()),
        // 마인크래프트 언어 코드
        "en_us" | "en_gb" => Some("en".into()),
        "ko_kr" => Some("ko".into()),
        "ja_jp" => Some("ja".into()),
        "zh_cn" => Some("zh-cn".into()),
        _ => None,
    }
}

/// 아카이브 내부 파일 경로에서 언어 힌트 감지
fn detect_archive_entry_language(entry_path: &str) -> Option<String> {
    let path = Path::new(entry_path);
    
    // 파일명에서 언어 코드 추출 (예: en_us.json, ko_kr.json)
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        let lower = stem.to_lowercase();
        if let Some(code) = normalize_language_code(&lower) {
            return Some(code);
        }
    }
    
    // 경로의 각 부분에서 언어 코드 탐색
    for component in entry_path.split('/') {
        let lower = component.to_lowercase();
        for token in split_language_tokens(&lower) {
            if let Some(code) = normalize_language_code(&token) {
                return Some(code);
            }
        }
    }
    
    None
}

fn file_name_to_string(path: &Path) -> Result<String, String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("파일 이름을 확인할 수 없습니다: {:?}", path))?;
    os_str_to_string(file_name)
}

fn os_str_to_string(value: &OsStr) -> Result<String, String> {
    value
        .to_str()
        .map(|string| string.to_string())
        .ok_or_else(|| "파일 이름을 UTF-8로 변환할 수 없습니다.".into())
}

fn resolve_workshop_title(steamapps: &Path, app_id: &str, mod_id: &str) -> Option<String> {
    let workshop_file = steamapps.join(format!("workshop/appworkshop_{}.acf", app_id));
    let contents = fs::read_to_string(workshop_file).ok()?;
    let pattern = format!(
        r#"(?s)"{}"\s*\{{.*?"title"\s*"([^"]+)""#,
        regex::escape(mod_id)
    );
    let regex = Regex::new(&pattern).ok()?;
    let captures = regex.captures(&contents)?;
    let raw = captures.get(1)?.as_str();
    clean_title(raw)
}

fn resolve_about_metadata_title(mod_path: &Path) -> Option<String> {
    let about_path = mod_path.join("About/About.xml");
    let contents = fs::read_to_string(about_path).ok()?;
    let captures = RIMWORLD_ABOUT_NAME_CAPTURE.captures(&contents)?;
    let raw = captures.get(1)?.as_str();
    clean_title(raw)
}

fn clean_title(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

