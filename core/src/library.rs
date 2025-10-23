use crate::policy::PolicyProfile;
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

#[derive(Debug, Default)]
pub struct LibraryScanner;

impl LibraryScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan(&self, candidates: &[PathBuf]) -> Result<Vec<LibraryEntry>, String> {
        if candidates.is_empty() {
            return Ok(vec![self.placeholder_entry("<no library paths detected>")]);
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
                    "steamapps directory not found; add the library manually from settings.".into(),
                );
            } else if mods.is_empty() {
                notes.push("No workshop content detected. Launch Steam once to let it generate appworkshop manifests.".into());
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
            notes: vec!["Provide a Steam library path to begin scanning.".into()],
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
                    "Workshop content unreadable",
                    "Unknown Game",
                    vec!["en".into()],
                    format!("Failed to enumerate workshop content: {err}"),
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
                    .unwrap_or_else(|| "unknown".into());

                let languages = self.detect_languages(&mod_dir.path());
                let warnings = self.collect_warnings(&mod_dir.path());

                mods.push(ModSummary {
                    id: format!("{app_id}:{mod_id}"),
                    name: format!("Workshop Item {mod_id}"),
                    game: format!("App {app_id}"),
                    installed_languages: languages,
                    last_updated,
                    policy: PolicyProfile::conservative(format!("App {app_id}")),
                    warnings,
                });
            }
        }

        if mods.is_empty() {
            mods.push(self.synthetic_mod(
                "no-content",
                "No workshop archives detected",
                "Unknown Game",
                vec!["en".into()],
                "Run a Steam sync to populate workshop/content.".into(),
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
            last_updated: "unknown".into(),
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
            warnings
                .push("Managed DLL detected – prefer resource extraction over decompiling.".into());
        }
        if contains_binary {
            warnings.push(
                "Unity AssetBundle detected – skipped by default per workspace policy.".into(),
            );
        }

        warnings
    }
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
        Err(_) => "unknown".into(),
    }
}
