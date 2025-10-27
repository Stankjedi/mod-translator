use crate::policy::{self, PolicyBanner, PolicyProfile};
use crate::steam::{resolve_app_name, SteamLocator};
use crate::time::{format_system_time, FormattedTimestamp};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
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
    pub installed_languages: Vec<String>,
    pub last_updated: FormattedTimestamp,
    pub policy: PolicyProfile,
    pub warnings: Vec<String>,
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
}

#[derive(Debug, Default)]
pub struct LibraryScanner;

impl LibraryScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan(&self, candidates: &[PathBuf]) -> Result<Vec<LibraryEntry>, String> {
        if candidates.is_empty() {
            return Ok(vec![
                self.placeholder_entry("라이브러리 경로를 찾을 수 없음")
            ]);
        }

        let mut entries = Vec::new();
        for root in candidates {
            let steamapps = root.join("steamapps");
            let exists = steamapps.exists();
            let workshop_root = steamapps.join("workshop");
            let mut workshop_display = None;
            let mut notes = Vec::new();

            let mods = if exists {
                match self.detect_workshop_mods(&steamapps) {
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
                notes.push("워크샵 콘텐츠를 찾지 못했습니다. Steam을 한 번 실행하여 appworkshop 정보를 생성하세요.".into());
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

    fn detect_workshop_mods(&self, steamapps: &Path) -> Result<Vec<ModSummary>, String> {
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
            if !app_dir.path().is_dir() {
                continue;
            }

            let app_id = match file_name_to_string(&app_dir.path()) {
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

            let mod_entries = match fs::read_dir(app_dir.path()) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for mod_dir in mod_entries.flatten() {
                if !mod_dir.path().is_dir() {
                    continue;
                }

                let mod_path = mod_dir.path();

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

                mods.push(ModSummary {
                    id: format!("{app_id}:{mod_id}"),
                    name: resolved_name,
                    game: game_name.clone(),
                    installed_languages: languages,
                    last_updated,
                    policy: PolicyProfile::conservative(game_name.clone()),
                    warnings,
                });
            }
        }

        if mods.is_empty() {
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

    let candidates = locator.library_candidates(primary_path.as_deref());
    let libraries = scanner.scan(&candidates)?;

    Ok(LibraryScanResponse {
        libraries,
        policy_banner: policy::default_policy_banner(),
    })
}

fn to_utf8_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(|value| value.to_string())
        .ok_or_else(|| format!("경로를 UTF-8 문자열로 변환하지 못했습니다: {:?}", path))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

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
