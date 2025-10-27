use crate::policy::{self, PolicyBanner, PolicyProfile};
use crate::steam::SteamLocator;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LibraryStatus {
    Healthy,
    Missing,
}

#[derive(Debug, Serialize, Clone)]
pub struct ModSummary {
    pub id: String,
    pub name: String,
    pub game: String,
    pub installed_languages: Vec<String>,
    pub last_updated: String,
    pub policy: PolicyProfile,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct LibraryEntry {
    pub path: String,
    pub status: LibraryStatus,
    pub mods: Vec<ModSummary>,
    pub workshop_root: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
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
            let workshop_display = workshop_root
                .exists()
                .then(|| workshop_root.to_string_lossy().to_string());

            let mods = if exists {
                self.detect_workshop_mods(&steamapps)
            } else {
                Vec::new()
            };

            let mut notes = Vec::new();
            if !exists {
                notes.push(
                    "steamapps 디렉터리를 찾을 수 없습니다. 설정에서 라이브러리를 수동으로 추가하세요.".into(),
                );
            } else if mods.is_empty() {
                notes.push("워크샵 콘텐츠를 찾지 못했습니다. Steam을 한 번 실행하여 appworkshop 정보를 생성하세요.".into());
            }

            entries.push(LibraryEntry {
                path: root.to_string_lossy().to_string(),
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

    fn detect_workshop_mods(&self, steamapps: &Path) -> Vec<ModSummary> {
        let mut mods = Vec::new();
        let content_root = steamapps.join("workshop/content");
        if !content_root.exists() {
            return mods;
        }

        let app_dirs = match fs::read_dir(&content_root) {
            Ok(entries) => entries,
            Err(err) => {
                return vec![self.synthetic_mod(
                    "unknown-app",
                    "워크샵 콘텐츠를 읽을 수 없습니다",
                    "알 수 없는 게임",
                    vec!["en".into()],
                    format!("워크샵 콘텐츠를 열거하지 못했습니다: {err}"),
                )];
            }
        };

        for app_dir in app_dirs.flatten() {
            let app_id = app_dir.file_name().to_string_lossy().to_string();
            if !app_dir.path().is_dir() {
                continue;
            }

            let mod_entries = match fs::read_dir(app_dir.path()) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for mod_dir in mod_entries.flatten() {
                if !mod_dir.path().is_dir() {
                    continue;
                }

                let mod_id = mod_dir.file_name().to_string_lossy().to_string();
                let metadata = fs::metadata(mod_dir.path()).ok();
                let last_updated = metadata
                    .and_then(|meta| meta.modified().ok())
                    .map(|time| format_timestamp(time))
                    .unwrap_or_else(|| "알 수 없음".into());

                let languages = self.detect_languages(&mod_dir.path());
                let warnings = self.collect_warnings(&mod_dir.path());

                mods.push(ModSummary {
                    id: format!("{app_id}:{mod_id}"),
                    name: format!("워크샵 항목 {mod_id}"),
                    game: format!("앱 {app_id}"),
                    installed_languages: languages,
                    last_updated,
                    policy: PolicyProfile::conservative(format!("앱 {app_id}")),
                    warnings,
                });
            }
        }

        if mods.is_empty() {
            mods.push(self.synthetic_mod(
                "no-content",
                "워크샵 아카이브를 찾지 못했습니다",
                "알 수 없는 게임",
                vec!["en".into()],
                "Steam 동기화를 실행하여 workshop/content를 채워 주세요.".into(),
            ));
        }

        mods
    }

    fn synthetic_mod(
        &self,
        mod_id: &str,
        name: &str,
        game: &str,
        languages: Vec<String>,
        warning: String,
    ) -> ModSummary {
        ModSummary {
            id: format!("{}-{}", mod_id, Uuid::new_v4()),
            name: name.into(),
            game: game.into(),
            installed_languages: languages,
            last_updated: "알 수 없음".into(),
            policy: PolicyProfile::conservative(game),
            warnings: vec![warning],
        }
    }

    fn detect_languages(&self, path: &Path) -> Vec<String> {
        let mut detected = vec!["en".into()];
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
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
                let name = entry.file_name().to_string_lossy().to_lowercase();
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
                .map(|path| path.to_string_lossy().to_string())
        });

    let candidates = locator.library_candidates(primary_path.as_deref());
    let libraries = scanner.scan(&candidates)?;

    Ok(LibraryScanResponse {
        libraries,
        policy_banner: policy::default_policy_banner(),
    })
}

fn format_timestamp(time: SystemTime) -> String {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let seconds = duration.as_secs();
            format!(
                "{}",
                chrono::NaiveDateTime::from_timestamp_opt(seconds as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| seconds.to_string())
            )
        }
        Err(_) => "알 수 없음".into(),
    }
}
